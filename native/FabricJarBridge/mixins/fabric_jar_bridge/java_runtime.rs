//! Experimental, opt-in Java sidecar: real Fabric onInitialize bytecode execution.
//! Only net.fabricmc.api.* entrypoint interfaces and HorizonBridge.registerCommand
//! are implemented. No Minecraft Java classes, Fabric API, or Mixin support.
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const MAX_ENTRYPOINTS: usize = 8;
const MAX_COMMANDS: usize = 32;
#[derive(Clone)]
struct Found { id: String, sha: String }
#[derive(Default)]
struct State { found: Vec<Found>, commands: BTreeMap<String,String> }
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
fn state() -> &'static Mutex<State> { STATE.get_or_init(||Mutex::new(State::default())) }
pub fn reset() { if let Ok(mut s)=state().lock() {*s=State::default();} }
pub fn command(name: &str) -> Option<String> {state().lock().ok()?.commands.get(name).cloned()}
fn trust_path() -> PathBuf {
    std::env::current_dir().unwrap_or_default().join("horizon-mods/fabricbridge-trusted.txt")
}
fn trusted(id:&str,sha:&str)->bool {
    fs::read_to_string(trust_path()).unwrap_or_default().lines()
        .any(|line|line.split_once(' ').is_some_and(|(a,b)|a==id && b.trim()==sha))
}
pub fn trust(id:&str)->String {
    if !valid_id(id) {return "Identifiant Fabric invalide.".into();}
    let found=match state().lock(){
        Ok(s)=>s.found.iter().filter(|f|f.id==id).cloned().collect::<Vec<_>>(),
        Err(_)=>return "Etat du bridge indisponible.".into(),
    };
    if found.len()!=1 {return format!("{id}: besoin d'un seul JAR ({} detectes).",found.len());}
    if trusted(id,&found[0].sha) {return format!("{id} deja autorise, redemarre Horizon.");}
    let mut file=match fs::OpenOptions::new().create(true).append(true).open(trust_path()){
        Ok(f)=>f,Err(e)=>return format!("Ecriture autorisation impossible: {e}"),
    };
    if let Err(e)=writeln!(file,"{id} {}",found[0].sha) {
        return format!("Ecriture autorisation impossible: {e}");
    }
    format!("{id}: Java AUTORISE pour ce JAR (SHA-256). Redemarre; attention: acces PC complet.")
}
fn valid_id(s:&str)->bool {
    !s.is_empty() && s.len()<=64
        && s.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b==b'_')
}
fn valid_class(s:&str)->bool {
    !s.is_empty() && s.len()<=180 && !s.starts_with('.') && !s.ends_with('.')
        && !s.contains("..")
        && s.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'.'||b==b'_'||b==b'$')
}
fn read_entrypoints(path:&Path)->Result<(String,Vec<(String,String)>),String>{
    let mut archive=ZipArchive::new(File::open(path).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())?;
    let mut text=String::new();
    let file=archive.by_name("fabric.mod.json").map_err(|_|"fabric.mod.json absent".to_owned())?;
    if file.size()>65536 {return Err("manifest trop volumineux".into());}
    file.take(65537).read_to_string(&mut text).map_err(|e|e.to_string())?;
    let meta:serde_json::Value=serde_json::from_str(&text).map_err(|e|e.to_string())?;
    let id=meta["id"].as_str().unwrap_or("");
    if !valid_id(id) {return Err("id Fabric non pris en charge".into());}
    let mut entries=Vec::new();
    for kind in ["main","client"] {
        if let Some(values)=meta["entrypoints"][kind].as_array(){
            for v in values {
                let class=v.as_str().or_else(||{
                    if v["adapter"].as_str().is_none_or(|a|a=="default") {
                        v["value"].as_str()
                    } else {None}
                });
                if let Some(c)=class.filter(|c|valid_class(c)) {
                    entries.push((kind.to_owned(),c.to_owned()));
                }
            }
        }
    }
    if entries.len()>MAX_ENTRYPOINTS {return Err("trop d'entrypoints".into());}
    Ok((id.to_owned(),entries))
}
fn sha(path:&Path)->Result<String,String>{
    let mut file=File::open(path).map_err(|e|e.to_string())?;
    let mut hash=Sha256::new();
    let mut buf=[0u8;65536];
    loop {
        let n=file.read(&mut buf).map_err(|e|e.to_string())?;
        if n==0 {break;}
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}",hash.finalize()))
}
fn timed(mut cmd:Command, limit:Duration)->Result<Output,String>{
    let mut child=cmd.stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e|format!("Java/JDK indisponible: {e}"))?;
    let start=Instant::now();
    loop {
        if child.try_wait().map_err(|e|e.to_string())?.is_some() {
            return child.wait_with_output().map_err(|e|e.to_string());
        }
        if start.elapsed()>limit {
            let _=child.kill();
            let _=child.wait();
            return Err(format!("delai Java depasse ({}s)",limit.as_secs()));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

// Source-compatible Fabric interfaces and a deliberately tiny Horizon API.
// Compiled from sources on first approved execution, using user's local JDK.
const MOD: &str="package net.fabricmc.api; public interface ModInitializer {void onInitialize();}\n";
const CLIENT: &str="package net.fabricmc.api; public interface ClientModInitializer {void onInitializeClient();}\n";
const API: &str=r#"package dev.horizon.fabricbridge;
public final class HorizonBridge {
    private HorizonBridge() {}
    public static void registerCommand(String name, String reply) {
        if(name==null || !name.matches("[a-z][a-z0-9_]{0,31}"))
            throw new IllegalArgumentException("Invalid command name");
        if(reply==null) throw new IllegalArgumentException("Null reply");
        reply=reply.replace('\n',' ').replace('\r',' ').replace('\t',' ');
        if(reply.length()>180) reply=reply.substring(0,180);
        System.out.println("HBR1\t"+name+"\t"+reply);
        System.out.flush();
    }
}
"#;
const RUNNER: &str=r#"package dev.horizon.fabricbridge;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.api.ClientModInitializer;
public final class Runner {
    public static void main(String[] args) throws Exception {
        if(args.length!=2) throw new IllegalArgumentException("mode and class required");
        Class<?> type=Class.forName(args[1]);
        Object mod=type.getDeclaredConstructor().newInstance();
        if("main".equals(args[0])) ((ModInitializer)mod).onInitialize();
        else if("client".equals(args[0])) ((ClientModInitializer)mod).onInitializeClient();
        else throw new IllegalArgumentException("Unsupported Fabric entrypoint");
    }
}
"#;
fn shim_files()->[(&'static str,&'static str);4]{
    [
        ("net/fabricmc/api/ModInitializer.java",MOD),
        ("net/fabricmc/api/ClientModInitializer.java",CLIENT),
        ("dev/horizon/fabricbridge/HorizonBridge.java",API),
        ("dev/horizon/fabricbridge/Runner.java",RUNNER),
    ]
}
fn shim()->Result<PathBuf,String>{
    let dir=std::env::current_dir().unwrap_or_default()
        .join(".horizon-cache/fabric-jar-bridge/java-shim");
    let classes=dir.join("classes");
    if classes.join("dev/horizon/fabricbridge/Runner.class").is_file() {return Ok(classes);}
    let mut files=Vec::new();
    for (path,source) in shim_files(){
        let file=dir.join(path);
        fs::create_dir_all(file.parent().unwrap_or(&dir)).map_err(|e|e.to_string())?;
        fs::write(&file,source).map_err(|e|e.to_string())?;
        files.push(file);
    }
    fs::create_dir_all(&classes).map_err(|e|e.to_string())?;
    let mut cmd=Command::new("javac");
    cmd.arg("-encoding").arg("UTF-8").arg("-d").arg(&classes).args(&files);
    let output=timed(cmd,Duration::from_secs(20))
        .map_err(|e|format!("{e}; JDK avec javac requis"))?;
    if !output.status.success(){
        return Err(format!("javac: {}",String::from_utf8_lossy(&output.stderr)
            .lines().next().unwrap_or("compilation echouee")));
    }
    Ok(classes)
}
fn execute(jar:&Path,entries:&[(String,String)])->Result<Vec<(String,String)>,String>{
    let classes=shim()?;
    let separator=if cfg!(windows){';'}else{':'};
    let classpath=format!("{}{}{}",classes.display(),separator,jar.display());
    let mut commands=Vec::new();
    for (kind,class) in entries {
        let mut cmd=Command::new("java");
        cmd.arg("-cp").arg(&classpath).arg("dev.horizon.fabricbridge.Runner")
            .arg(kind).arg(class);
        let output=timed(cmd,Duration::from_secs(6))?;
        if !output.status.success(){
            return Err(format!("{kind} {class}: {}",String::from_utf8_lossy(&output.stderr)
                .lines().next().unwrap_or("initialisation echouee")));
        }
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let Some(body)=line.strip_prefix("HBR1\t") else {continue;};
            let Some((name,reply))=body.split_once('\t') else {continue;};
            if name.is_empty() || name.len()>32
                || !name.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b==b'_')
                || reply.len()>720 || commands.len()>=MAX_COMMANDS {continue;}
            commands.push((name.to_string(),reply.to_string()));
        }
    }
    Ok(commands)
}
pub fn inspect_and_run(jar:&Path)->String{
    let (id,entries)=match read_entrypoints(jar){
        Ok(x)=>x,Err(e)=>return format!("Java: {e}"),
    };
    if entries.is_empty(){return "Java: pas d'entrypoint main/client compatible".into();}
    let hash=match sha(jar){Ok(x)=>x,Err(e)=>return format!("SHA-256: {e}")};
    if let Ok(mut s)=state().lock(){s.found.push(Found{id:id.clone(),sha:hash.clone()});}
    if !trusted(&id,&hash) {
        return format!("Java NON execute (securite). /fabricbridge trust {id}, puis redemarrage");
    }
    match execute(jar,&entries){
        Ok(commands)=>{
            let count=commands.len();
            if let Ok(mut s)=state().lock(){
                for (name,reply) in commands {
                    if s.commands.len()<MAX_COMMANDS {s.commands.entry(name).or_insert(reply);}
                }
            }
            format!("Java reel execute: {} entrypoints, {count} commandes Horizon",entries.len())
        }
        Err(e)=>format!("Java echec/incompatibilite: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::{ZipWriter,write::SimpleFileOptions};
    #[test]
    fn validates_fabric_classes(){
        assert!(valid_class("example.MyFabricMod"));
        assert!(!valid_class("example::init"));
        assert!(!valid_class("../evil"));
    }
    #[test]
    fn runs_actual_fabric_initializer_and_exposes_command(){
        if Command::new("javac").arg("-version").output().is_err(){return;}
        let root=std::env::temp_dir().join(format!("fabricbridge-java-test-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        let src=root.join("src");let classes=root.join("classes");
        fs::create_dir_all(&classes).unwrap();
        let mut files=Vec::new();
        for (name,text) in shim_files().into_iter().chain(std::iter::once((
            "test/ActualFabricMod.java",
            r#"package test;
public class ActualFabricMod implements net.fabricmc.api.ModInitializer {
    public void onInitialize() {
        dev.horizon.fabricbridge.HorizonBridge.registerCommand("javaproof","Java Fabric a execute son bytecode");
    }
}"#,
        ))){
            let p=src.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p,text).unwrap();files.push(p);
        }
        let compile=Command::new("javac").arg("-d").arg(&classes).args(&files).output().unwrap();
        assert!(compile.status.success(),"{}",String::from_utf8_lossy(&compile.stderr));
        let jar=root.join("example.jar");
        let mut w=ZipWriter::new(File::create(&jar).unwrap());
        w.start_file("fabric.mod.json",SimpleFileOptions::default()).unwrap();
        w.write_all(br#"{"id":"example","version":"1","entrypoints":{"main":["test.ActualFabricMod"]}}"#).unwrap();
        w.start_file("test/ActualFabricMod.class",SimpleFileOptions::default()).unwrap();
        w.write_all(&fs::read(classes.join("test/ActualFabricMod.class")).unwrap()).unwrap();
        w.finish().unwrap();
        let (id,eps)=read_entrypoints(&jar).unwrap();assert_eq!(id,"example");
        let cp=format!("{}{}{}",classes.display(),if cfg!(windows){';'}else{':'},jar.display());
        let out=Command::new("java").arg("-cp").arg(cp)
            .arg("dev.horizon.fabricbridge.Runner").arg(&eps[0].0).arg(&eps[0].1).output().unwrap();
        assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
        assert!(String::from_utf8_lossy(&out.stdout).contains("HBR1\tjavaproof\tJava Fabric a execute son bytecode"));
        let _=fs::remove_dir_all(root);
    }
}
