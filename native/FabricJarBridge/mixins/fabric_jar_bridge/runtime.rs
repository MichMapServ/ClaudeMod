// Fabric Jar Bridge: carries *data* from local Fabric jars into Horizon's
// content overlay. Never loads Java bytecode, mixin classes, native libraries,
// network code, or Fabric API. See README for exact compatibility.
use std::{
    collections::{BTreeSet, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use horizon_client::{mod_content::{self, ContentMod}, mod_list};
use horizon_mods::{Capability, Content, Package};
use sha2::{Digest, Sha256};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub(super) const ID: &str = "fabric_jar_bridge";
const MAX_JARS: usize = 64;
const MAX_ENTRIES: usize = 4096;
const MAX_ITEM: u64 = 8 * 1024 * 1024;
const MAX_TOTAL: u64 = 64 * 1024 * 1024;
const MAX_JAR: u64 = 128 * 1024 * 1024;
static RESULTS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn results() -> &'static Mutex<Vec<String>> {
    RESULTS.get_or_init(|| Mutex::new(Vec::new()))
}
fn publish(lines: Vec<String>) {
    if let Ok(mut out) = results().lock() { *out = lines; }
}
pub(super) fn status_lines() -> Vec<String> {
    let mut out = vec!["Fabric Bridge: ressources et Java experimental avec autorisation explicite par JAR.".to_owned()];
    if let Ok(lines) = results().lock() {
        if lines.is_empty() { out.push("Aucun fichier .jar detecte dans horizon-mods/.".into()); }
        else { out.extend(lines.iter().take(12).cloned()); }
        if lines.len() > 12 { out.push(format!("... {} autres jars (voir le journal).", lines.len()-12)); }
    }
    out
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 &&
    id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn carry_path(name: &str, id: &str) -> bool {
    // Asset-only import: do not move embedded executables, Java classes or
    // nested archives even if a malicious jar disguises them as resources.
    if [".class",".jar",".dll",".exe",".so",".dylib",".ps1",".bat",".sh",".wasm"].iter()
        .any(|ext|name.ends_with(ext)) {return false;}
    if horizon_mods::content::check_path(name).is_err() { return false; }
    let Some((top, ns, rest)) = ({
        let mut p = name.splitn(3, '/');
        p.next().zip(p.next()).and_then(|(a,b)| p.next().map(|c| (a,b,c)))
    }) else { return false; };
    let owns = ns == id;
    owns && matches!(top, "assets" | "data")
        || top == "data" && (ns == "minecraft" || ns == "c") && rest.starts_with("tags/")
}
fn metadata(zip: &mut ZipArchive<File>) -> Result<(String, String), String> {
    let mut text = String::new();
    {
        let f = zip.by_name("fabric.mod.json").map_err(|_| "ce JAR ne contient pas fabric.mod.json".to_owned())?;
        if f.size() > 65_536 { return Err("fabric.mod.json trop volumineux".into()); }
        f.take(65_537).read_to_string(&mut text).map_err(|e| e.to_string())?;
    }
    let obj: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("fabric.mod.json: {e}"))?;
    let id = obj.get("id").and_then(|s|s.as_str()).unwrap_or("");
    if !valid_id(id) { return Err(format!("identifiant Fabric {id:?} non pris en charge")); }
    let version = obj.get("version").and_then(|s|s.as_str()).unwrap_or("?").chars().take(24).collect();
    Ok((id.to_owned(), version))
}
fn digest(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e|e.to_string())?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 65_536];
    loop {
        let n = f.read(&mut buf).map_err(|e|e.to_string())?;
        if n == 0 {break;}
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
// The conversion is intentionally narrow: no class files, FML metadata,
// executable scripts, platform overrides or arbitrary filesystem extraction.
fn convert_jar(path: &Path, cache: &Path) -> Result<Option<(PathBuf, String)>, String> {
    let size = fs::metadata(path).map_err(|e|e.to_string())?.len();
    if size > MAX_JAR { return Err("JAR depasse la limite de 128 Mio".into()); }
    let file = File::open(path).map_err(|e|e.to_string())?;
    let mut jar = ZipArchive::new(file).map_err(|e|format!("JAR invalide: {e}"))?;
    if jar.len() > MAX_ENTRIES {return Err("trop d'entrees dans le JAR".into());}
    let (id, version) = metadata(&mut jar)?;
    let mut accepted: Vec<(String, Vec<u8>)> = Vec::new();
    let mut skipped = 0usize;
    let mut classes = 0usize;
    let mut total = 0u64;
    let mut names = HashSet::new();
    for i in 0..jar.len() {
        let mut file = jar.by_index(i).map_err(|e|e.to_string())?;
        let name = file.name().to_owned();
        if name.ends_with(".class") {classes+=1;}
        if file.is_dir() || !name.starts_with("assets/") && !name.starts_with("data/") {
            continue;
        }
        if !carry_path(&name, &id) {skipped+=1;continue;}
        if !names.insert(name.clone()) {return Err(format!("entree dupliquee: {name}"));}
        if file.size()>MAX_ITEM || total.saturating_add(file.size())>MAX_TOTAL {
            return Err("contenu dezippe trop volumineux".into());
        }
        if file.compressed_size()>0 && file.size()/file.compressed_size()>200 {
            return Err("ratio de compression suspect".into());
        }
        if file.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) {
            return Err("lien symbolique refuse".into());
        }
        let mut bytes = Vec::new();
        file.by_ref().take(MAX_ITEM+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        if bytes.len() as u64 > MAX_ITEM {return Err("fichier decompresse trop volumineux".into());}
        total=total.saturating_add(bytes.len() as u64);
        accepted.push((name,bytes));
    }
    if accepted.is_empty() {
        return Ok(None);
    }
    accepted.sort_by(|a,b|a.0.cmp(&b.0));
    let sha=digest(path)?;
    let out=cache.join(format!("{id}-{}.hmod", &sha[..16]));
    if !out.is_file() {
        fs::create_dir_all(cache).map_err(|e|e.to_string())?;
        let tmp=cache.join(format!(".{id}-{}-{}.tmp", &sha[..16], std::process::id()));
        let res=(||->Result<(),String>{
            let mut zip=ZipWriter::new(File::create(&tmp).map_err(|e|e.to_string())?);
            let opts=SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            let manifest=format!("id = \"{id}\"\nname = \"Fabric data: {id}\"\nversion = \"0.1.0\"\napi = \"0.2\"\ncapabilities = [\"registries\"]\n");
            zip.start_file("horizon-mod.toml",opts).map_err(|e|e.to_string())?;
            zip.write_all(manifest.as_bytes()).map_err(|e|e.to_string())?;
            for (name,bytes) in &accepted {
                zip.start_file(name,opts).map_err(|e|e.to_string())?;
                zip.write_all(bytes).map_err(|e|e.to_string())?;
            }
            zip.finish().map_err(|e|e.to_string())?;
            fs::rename(&tmp,&out).map_err(|e|e.to_string())?;
            Ok(())
        })();
        if res.is_err(){let _=fs::remove_file(&tmp);}
        res?;
    }
    let status=format!("{id} {version}: {} ressource(s) importee(s), {classes} classe(s) Java ignoree(s), {skipped} entree(s) hors perimetre.",accepted.len());
    Ok(Some((out,status)))
}
// This startup hook is intentionally tied to the exact Horizon source commit.
// It reproduces Horizon's original content overlay discovery and adds only
// validated carried JAR resources, preserving already-installed .hmod files.
pub(super) fn content_root(root: &Path, enabled: &BTreeSet<String>, dev: &[PathBuf]) -> PathBuf {
    let here = std::env::current_dir().unwrap_or_default();
    let dir = here.join(mod_list::DIR);
    let mut paths = horizon_mods::find(&dir);
    paths.retain(|p| !dev.contains(p));
    paths.extend(dev.iter().cloned());
    let mut mods: Vec<ContentMod> = Vec::new();
    for path in &paths {
        let Ok(package)=Package::read(path) else {continue;};
        let m=&package.manifest;
        if !(enabled.contains(&m.id)||dev.contains(path))
            || !m.capabilities.contains(&Capability::Registries)
            || mods.iter().any(|c|c.id==m.id) {continue;}
        let parsed=Content::read(path,m).and_then(|content|{
            let own=(m.id.as_str(),content.namespaces.as_slice());
            match mods.iter().find_map(|c|horizon_mods::content::clash((&c.id,&c.content.namespaces),own)){
                Some(e)=>Err(e),None=>Ok(content)
            }
        });
        match parsed {
            Ok(content)=>mods.push(ContentMod{id:m.id.clone(),content}),
            Err(e)=>log::warn!("Fabric Bridge: standard mod {} refused: {e}",m.id),
        }
    }
    let original_mods=mods.len();
    let mut lines=Vec::new();
    let mut jars=fs::read_dir(&dir).ok().into_iter().flatten().flatten()
        .map(|e|e.path())
        .filter(|p|p.extension().is_some_and(|x|x.eq_ignore_ascii_case("jar")) && p.is_file())
        .collect::<Vec<_>>();
    jars.sort();
    if jars.len()>MAX_JARS {
        lines.push(format!("Seulement {MAX_JARS} JARs scannes ({} ignores).",jars.len()-MAX_JARS));
        jars.truncate(MAX_JARS);
    }
    let cache=here.join(".horizon-cache/fabric-jar-bridge");
    crate::game::fabric_java_bridge::reset();
    // Resolve the full local Fabric mod set before starting Java, so
    // dependency classes share a single JVM and initialize in graph order.
    let java_reports=crate::game::fabric_java_bridge::inspect_and_run_all(&jars);
    for (jar,java_report) in jars.into_iter().zip(java_reports) {
        let name=jar.file_name().unwrap_or_default().to_string_lossy().to_string();
        match convert_jar(&jar,&cache) {
            Ok(Some((carried,summary)))=>{
                let parsed=Package::read(&carried).and_then(|p|{
                    let c=Content::read(&carried,&p.manifest)?;
                    if mods.iter().any(|m|m.id==p.manifest.id) {return Err("identifiant deja utilise".into());}
                    if let Some(e)=mods.iter().find_map(|m|horizon_mods::content::clash((&m.id,&m.content.namespaces),(&p.manifest.id,&c.namespaces))) {
                        return Err(e);
                    }
                    Ok(ContentMod{id:p.manifest.id,content:c})
                });
                match parsed {
                    Ok(c)=>{log::info!("Fabric Bridge: {name}: {summary}");mods.push(c);lines.push(summary);}
                    Err(e)=>{log::warn!("Fabric Bridge: {name}: {e}");lines.push(format!("{name}: refuse - {e}"));}
                }
            }
            Ok(None)=>{
                lines.push(format!("{name}: aucun contenu portable (voir statut Java)."));
            }
            Err(e)=>{
                log::warn!("Fabric Bridge: {name}: {e}");
                lines.push(format!("{name}: refuse - {e}"));
            }
        }
        lines.push(format!("{name}: {java_report}"));
    }
    match mod_content::resource_root(root,&mods,&here.join(".horizon-cache")){
        Ok(out)=>{publish(lines);out},
        Err(e)=>{
            log::warn!("Fabric Bridge: combined overlay failed: {e}; retrying original Horizon mods only");
            // A bad Fabric resource must never take down already installed .hmod content.
            mods.truncate(original_mods);
            lines.push("Import annule: une ressource incompatible a ete refusee (voir logs).".into());
            publish(lines);
            match mod_content::resource_root(root,&mods,&here.join(".horizon-cache")){
                Ok(out)=>out,
                Err(e)=>{log::warn!("Fabric Bridge: base content also failed: {e}");root.to_owned()}
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn safe_namespaces_and_no_vanilla_overrides(){
        assert!(carry_path("assets/ruby/textures/block/ruby.png","ruby"));
        assert!(carry_path("data/ruby/recipe/test.json","ruby"));
        assert!(carry_path("data/minecraft/tags/item/ores.json","ruby"));
        assert!(!carry_path("assets/minecraft/textures/gui/widgets.png","ruby"));
        assert!(!carry_path("assets/other/textures/a.png","ruby"));
        assert!(!carry_path("assets/ruby/../../evil.txt","ruby"));
        assert!(!carry_path("data/ruby/Bad.txt","ruby"));
        assert!(!carry_path("assets/ruby/lib/Hidden.class","ruby"));
        assert!(!carry_path("assets/ruby/native/code.dll","ruby"));
    }
    #[test] fn ids_are_checked(){
        assert!(valid_id("ruby_items"));
        assert!(!valid_id("fabric-api"));
        assert!(!valid_id("../evil"));
        assert!(!valid_id(""));
    }
    #[test] fn metadata_from_real_fabric_jar_format(){
        let temp=std::env::temp_dir().join(format!("horizon-fabricbridge-test-{}",std::process::id()));
        let _=fs::create_dir_all(&temp);
        let jar=temp.join("test.jar");
        {
            let mut w=ZipWriter::new(File::create(&jar).unwrap());
            let opts=SimpleFileOptions::default();
            w.start_file("fabric.mod.json",opts).unwrap();
            w.write_all(br#"{"schemaVersion":1,"id":"bridge_test","version":"1.0"}"#).unwrap();
            w.start_file("assets/bridge_test/lang/en_us.json",opts).unwrap();
            w.write_all(br#"{"item.bridge_test.foo":"Foo"}"#).unwrap();
            w.start_file("unsafe/Bad.class",opts).unwrap();
            w.write_all(b"CAFE").unwrap();
            w.finish().unwrap();
        }
        let cache=temp.join("cache");
        let (out,summary)=convert_jar(&jar,&cache).unwrap().unwrap();
        assert!(summary.contains("1 ressource(s)"));
        let pkg=Package::read(&out).unwrap();
        assert_eq!(pkg.manifest.id,"bridge_test");
        let content=Content::read(&out,&pkg.manifest).unwrap();
        assert_eq!(content.files,["assets/bridge_test/lang/en_us.json"]);
        assert_eq!(content.file("assets/bridge_test/lang/en_us.json").unwrap(),br#"{"item.bridge_test.foo":"Foo"}"#);
        let z=ZipArchive::new(File::open(&out).unwrap()).unwrap();
        assert!(!z.file_names().any(|n|n.ends_with(".class")));
        let _=fs::remove_dir_all(&temp);
    }
}
