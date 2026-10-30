// Experimental, opt-in Java sidecar: real Fabric onInitialize bytecode execution.
// A limited MinecraftClient/player position facade and END_CLIENT_TICK event
// are backed by real Horizon native tick hooks. No general Fabric/Minecraft/Mixin support.
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::{mpsc::{sync_channel, SyncSender, TrySendError}, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const MAX_ENTRYPOINTS: usize = 8;
const MAX_COMMANDS: usize = 32;
#[derive(Clone)]
struct Found { id: String, sha: String }
struct JavaProcess { child: Child, send: SyncSender<String> }
#[derive(Default)]
struct State {
    found: Vec<Found>,
    commands: BTreeMap<String,String>,
    processes: Vec<JavaProcess>,
    generation: u64,
}
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
fn state() -> &'static Mutex<State> { STATE.get_or_init(||Mutex::new(State::default())) }
pub fn reset() {
    if let Ok(mut s)=state().lock() {
        for process in &mut s.processes {let _=process.child.kill();let _=process.child.wait();}
        let next=s.generation.wrapping_add(1);
        *s=State::default();
        s.generation=next;
    }
}
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
        && s.bytes().next().is_some_and(|c|c.is_ascii_lowercase())
        && s.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b==b'_'||b==b'-')
}
fn valid_class(s:&str)->bool {
    !s.is_empty() && s.len()<=180 && !s.starts_with('.') && !s.ends_with('.')
        && !s.contains("..")
        && s.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'.'||b==b'_'||b==b'$')
}
fn read_entrypoints(path:&Path)->Result<(String,Vec<(String,String)>),String>{
    if fs::metadata(path).map_err(|e|e.to_string())?.len()>128*1024*1024 {
        return Err("JAR depasse 128 MiB".into());
    }
    let mut archive=ZipArchive::new(File::open(path).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())?;
    if archive.len()>4096 {return Err("plus de 4096 fichiers JAR".into());}
    let mut names=HashSet::new();
    let mut inflated=0u64;
    for index in 0..archive.len() {
        let entry=archive.by_index(index).map_err(|e|e.to_string())?;
        let name=entry.name();
        let path_ok = !name.is_empty() && name.len()<=512 && !name.starts_with('/')
            && !name.contains('\\') && !name.contains(':') && !name.contains('\0')
            && name.trim_end_matches('/').split('/').all(|part|!part.is_empty()&&part!="."&&part!="..");
        if !path_ok {return Err("nom de fichier JAR non sur".into());}
        if !names.insert(name.to_owned()) {return Err("entree ZIP dupliquee".into());}
        if entry.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) {
            return Err("lien symbolique refuse dans JAR".into());
        }
        if name.ends_with(".class") && entry.size()>16*1024*1024 {
            return Err("classe Java individuelle trop volumineuse".into());
        }
        // Metadata is checked before launching Java; an attacker must not be
        // able to hide a multi-GiB archive behind a tiny compressed ZIP.
        if entry.size()>64*1024*1024 {return Err("entree ZIP trop volumineuse".into());}
        inflated=inflated.saturating_add(entry.size());
        if inflated>256*1024*1024 {return Err("JAR decompresse depasse 256 MiB".into());}
        if entry.size()>0 && (entry.compressed_size()==0
            || entry.size()/entry.compressed_size()>500) {
            return Err("ratio de compression ZIP suspect".into());
        }
    }
    let mut text=String::new();
    let file=archive.by_name("fabric.mod.json").map_err(|_|"fabric.mod.json absent".to_owned())?;
    if file.size()>65536 {return Err("manifest trop volumineux".into());}
    file.take(65537).read_to_string(&mut text).map_err(|e|e.to_string())?;
    let meta:serde_json::Value=serde_json::from_str(&text).map_err(|e|e.to_string())?;
    if !meta.is_object() {return Err("fabric.mod.json doit etre un objet".into());}
    let id=meta["id"].as_str().unwrap_or("");
    if !valid_id(id) {return Err("id Fabric non pris en charge".into());}
    if let Some(environment)=meta.get("environment") {
        if !matches!(environment.as_str(),Some("*"|"client")) {
            return Err("environment non pris en charge (client requis)".into());
        }
    }
    // No Java class transformation, language adapters, dependency resolver,
    // or custom lifecycle: reject them instead of running a partial mod.
    if meta.get("mixins").is_some_and(|v|!v.as_array().is_some_and(|a|a.is_empty())) {
        return Err("mixins Fabric declares: transformation Java/Minecraft NON prise en charge".into());
    }
    if meta.get("accessWidener").is_some() {
        return Err("accessWidener declare: non pris en charge".into());
    }
    if meta.get("jars").is_some_and(|v|!v.as_array().is_some_and(|a|a.is_empty())) {
        return Err("JARs embarques declares: dependances non resolues".into());
    }
    if let Some(deps)=meta.get("depends") {
        let deps=deps.as_object().ok_or("depends doit etre un objet")?;
        for key in deps.keys() {
            if !matches!(key.as_str(),"minecraft"|"fabricloader"|"java") {
                return Err(format!("dependance non prise en charge: {key}"));
            }
        }
    }
    let mut entries=Vec::new();
    if let Some(raw)=meta.get("entrypoints") {
        let groups=raw.as_object().ok_or("entrypoints doit etre un objet")?;
        for (kind, values) in groups {
            let values=values.as_array().ok_or_else(||format!("entrypoint {kind}: liste requise"))?;
            if kind!="main" && kind!="client" {
                if !values.is_empty() {
                    return Err(format!("entrypoint {kind} non pris en charge"));
                }
                continue;
            }
            for value in values {
                let class=match value {
                    serde_json::Value::String(class)=>class.as_str(),
                    serde_json::Value::Object(obj)=>{
                        if obj.get("adapter").and_then(|v|v.as_str()).is_some_and(|a|a!="default") {
                            return Err(format!("entrypoint {kind}: adapter Java non pris en charge"));
                        }
                        obj.get("value").and_then(|v|v.as_str())
                            .ok_or_else(||format!("entrypoint {kind}: classe invalide"))?
                    }
                    _=>return Err(format!("entrypoint {kind}: format invalide")),
                };
                if !valid_class(class) {return Err(format!("entrypoint {kind}: classe invalide"));}
                let class_path=format!("{}.class",class.replace('.',"/"));
                if !names.contains(&class_path) {
                    return Err(format!("entrypoint {kind}: classe absente du JAR: {class}"));
                }
                entries.push((kind.to_owned(),class.to_owned()));
                if entries.len()>MAX_ENTRYPOINTS {
                    return Err("trop d'entrypoints".into());
                }
            }
        }
    }
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
// Continuously drain both pipes while javac runs. Waiting for process exit
// before reading a pipe deadlocks when the compiler emits more than the OS
// pipe buffer. Capture at most 64 KiB per pipe while draining all bytes.
fn drain_bounded<R: Read + Send + 'static>(mut source:R)->thread::JoinHandle<Vec<u8>> {
    thread::spawn(move ||{
        let mut captured=Vec::new();
        let mut block=[0u8;8192];
        loop {
            match source.read(&mut block) {
                Ok(0)|Err(_)=>break,
                Ok(n)=>{
                    let room=65536usize.saturating_sub(captured.len());
                    captured.extend_from_slice(&block[..n.min(room)]);
                }
            }
        }
        captured
    })
}
fn timed(mut cmd:Command, limit:Duration)->Result<Output,String>{
    let mut child=cmd.stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e|format!("Java/JDK indisponible: {e}"))?;
    let stdout=drain_bounded(child.stdout.take().ok_or("stdout JDK indisponible")?);
    let stderr=drain_bounded(child.stderr.take().ok_or("stderr JDK indisponible")?);
    let start=Instant::now();
    let status=loop {
        match child.try_wait() {
            Ok(Some(status))=>break status,
            Ok(None)=>{},
            Err(e)=>{
                let _=child.kill();
                let _=child.wait();
                return Err(e.to_string());
            }
        }
        if start.elapsed()>limit {
            let _=child.kill();
            let _=child.wait();
            return Err(format!("delai Java depasse ({}s)",limit.as_secs()));
        }
        thread::sleep(Duration::from_millis(50));
    };
    let stdout=stdout.join().map_err(|_|"lecture stdout JDK interrompue")?;
    let stderr=stderr.join().map_err(|_|"lecture stderr JDK interrompue")?;
    Ok(Output{status,stdout,stderr})
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
const BLOCK_POS:&str=r#"package net.minecraft.util.math;
public final class BlockPos {
    private final int x,y,z;
    public BlockPos(int x,int y,int z){this.x=x;this.y=y;this.z=z;}
    public int getX(){return x;}
    public int getY(){return y;}
    public int getZ(){return z;}
}
"#;
const PLAYER:&str=r#"package net.minecraft.client.network;
import net.minecraft.util.math.BlockPos;
public class ClientPlayerEntity {
    private final double x,y,z;
    public ClientPlayerEntity(double x,double y,double z){this.x=x;this.y=y;this.z=z;}
    public double getX(){return x;}
    public double getY(){return y;}
    public double getZ(){return z;}
    public int getBlockX(){return (int)Math.floor(x);}
    public int getBlockY(){return (int)Math.floor(y);}
    public int getBlockZ(){return (int)Math.floor(z);}
    public BlockPos getBlockPos(){return new BlockPos(getBlockX(),getBlockY(),getBlockZ());}
}
"#;
const MINECRAFT:&str=r#"package net.minecraft.client;
import net.minecraft.client.network.ClientPlayerEntity;
public final class MinecraftClient {
    private static final MinecraftClient INSTANCE=new MinecraftClient();
    public volatile ClientPlayerEntity player;
    private MinecraftClient(){}
    public static MinecraftClient getInstance(){return INSTANCE;}
    public static void __horizonUpdate(double x,double y,double z,boolean present){
        INSTANCE.player=present?new ClientPlayerEntity(x,y,z):null;
    }
}
"#;
const CLIENT_TICK:&str=r#"package net.fabricmc.fabric.api.client.event.lifecycle.v1;
import java.util.concurrent.CopyOnWriteArrayList;
import net.minecraft.client.MinecraftClient;
public final class ClientTickEvents {
    private ClientTickEvents(){}
    @FunctionalInterface public interface EndTick {void onEndTick(MinecraftClient client);}
    public interface EndTickEvent {void register(EndTick callback);}
    private static final CopyOnWriteArrayList<EndTick> END=new CopyOnWriteArrayList<>();
    public static final EndTickEvent END_CLIENT_TICK=callback->END.add(callback);
    public static void __horizonTick(){
        MinecraftClient client=MinecraftClient.getInstance();
        for(EndTick callback:END) callback.onEndTick(client);
    }
}
"#;
const RUNNER: &str=r#"package dev.horizon.fabricbridge;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.api.ClientModInitializer;
public final class Runner {
    public static void main(String[] args) throws Exception {
        if(args.length==0 || args.length%2!=0) throw new IllegalArgumentException("mode/class pairs required");
        for(int i=0;i<args.length;i+=2) {
            Class<?> type=Class.forName(args[i+1]);
            Object mod=type.getDeclaredConstructor().newInstance();
            if("main".equals(args[i])) ((ModInitializer)mod).onInitialize();
            else if("client".equals(args[i])) ((ClientModInitializer)mod).onInitializeClient();
            else throw new IllegalArgumentException("Unsupported Fabric entrypoint");
        }
        java.io.BufferedReader input=new java.io.BufferedReader(
            new java.io.InputStreamReader(System.in,java.nio.charset.StandardCharsets.UTF_8));
        for(String line;(line=input.readLine())!=null;) {
            if("NONE".equals(line)) {
                net.minecraft.client.MinecraftClient.__horizonUpdate(0,0,0,false);
            } else if(line.startsWith("TICK\t")) {
                String[] values=line.split("\t");
                if(values.length!=4) continue;
                double x=Double.parseDouble(values[1]);
                double y=Double.parseDouble(values[2]);
                double z=Double.parseDouble(values[3]);
                net.minecraft.client.MinecraftClient.__horizonUpdate(x,y,z,true);
            } else continue;
            net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents.__horizonTick();
        }
    }
}
"#;
fn shim_files()->[(&'static str,&'static str);8]{
    [
        ("net/fabricmc/api/ModInitializer.java",MOD),
        ("net/fabricmc/api/ClientModInitializer.java",CLIENT),
        ("dev/horizon/fabricbridge/HorizonBridge.java",API),
        ("dev/horizon/fabricbridge/Runner.java",RUNNER),
        ("net/minecraft/util/math/BlockPos.java",BLOCK_POS),
        ("net/minecraft/client/network/ClientPlayerEntity.java",PLAYER),
        ("net/minecraft/client/MinecraftClient.java",MINECRAFT),
        ("net/fabricmc/fabric/api/client/event/lifecycle/v1/ClientTickEvents.java",CLIENT_TICK),
    ]
}
fn shim()->Result<PathBuf,String>{
    let dir=std::env::current_dir().unwrap_or_default()
        .join(".horizon-cache/fabric-jar-bridge/java-shim-v03");
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
fn parse_command(line:&str)->Option<(String,String)>{
    let payload=line.strip_prefix("HBR1\t")?;
    let (name,reply)=payload.split_once('\t')?;
    if name.is_empty() || name.len()>32
        || !name.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b==b'_')
        || reply.len()>720 {return None;}
    Some((name.to_owned(),reply.to_owned()))
}
fn execute(jar:&Path,entries:&[(String,String)])->Result<(),String>{
    let classes=shim()?;
    let separator=if cfg!(windows){';'}else{':'};
    let classpath=format!("{}{}{}",classes.display(),separator,jar.display());
    let mut cmd=Command::new("java");
    cmd.arg("-cp").arg(&classpath).arg("dev.horizon.fabricbridge.Runner");
    for (kind,class) in entries {cmd.arg(kind).arg(class);}
    let mut child=cmd.stdin(Stdio::piped()).stdout(Stdio::piped())
        .stderr(Stdio::inherit()).spawn().map_err(|e|format!("Java non disponible: {e}"))?;
    let stdin=child.stdin.take().ok_or("stdin JVM indisponible")?;
    let stdout=child.stdout.take().ok_or("stdout JVM indisponible")?;
    let (send,receive)=sync_channel::<String>(4);
    let generation=state().lock().map_err(|e|e.to_string())?.generation;
    thread::spawn(move ||{
        // Dedicated writer prevents slow Java mods from blocking the game tick.
        let mut out=stdin;
        for event in receive {
            if writeln!(out,"{event}").is_err() || out.flush().is_err(){break;}
        }
    });
    thread::spawn(move ||{
        // Java output is always untrusted. Read in bounded 2048-byte chunks.
        let mut reader=BufReader::new(stdout);
        // Do not use read_until: a malicious JAR can print an unbounded line
        // without a newline and exhaust Horizon's memory.
        let mut buffer=Vec::with_capacity(256);
        loop{
            buffer.clear();
            let mut too_long=false;
            loop {
                let Ok(chunk)=reader.fill_buf() else {return;};
                if chunk.is_empty() {return;}
                let newline=chunk.iter().position(|c|*c==b'\n');
                let count=newline.map_or(chunk.len(),|i|i+1);
                if !too_long {
                    if count<=2048usize.saturating_sub(buffer.len()) {
                        buffer.extend_from_slice(&chunk[..count]);
                    } else {too_long=true;}
                }
                reader.consume(count);
                if newline.is_some() {break;}
            }
            if !too_long {
                if let Ok(line)=std::str::from_utf8(&buffer) {;
                    if let Some((name,reply))=parse_command(line.trim_end_matches(['\r','\n'])) {
                        if let Ok(mut s)=state().lock(){
                            if s.generation==generation && s.commands.len()<MAX_COMMANDS {
                                // Do not allow a JAR to shadow Horizon's own commands
                                // or the bridge control endpoint.
                                if !matches!(name.as_str(),"fabricbridge"|"help"|"list"|"trust"){
                                    s.commands.entry(name).or_insert(reply);
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    state().lock().map_err(|e|e.to_string())?.processes.push(JavaProcess{child,send});
    Ok(())
}
/// Queue a client tick and actual player position for each running JVM.
/// Bounded channels avoid blocking Horizon when Java stalls.
pub fn tick(position:Option<[f64;3]>) {
    let line=match position {
        Some([x,y,z]) if x.is_finite() && y.is_finite() && z.is_finite()=>format!("TICK\t{x}\t{y}\t{z}"),
        _=>"NONE".to_owned(),
    };
    if let Ok(mut s)=state().lock(){
        s.processes.retain_mut(|p|{
            if !matches!(p.child.try_wait(),Ok(None)){return false;}
            match p.send.try_send(line.clone()){
                Ok(())|Err(TrySendError::Full(_))=>true,
                Err(TrySendError::Disconnected(_))=>false,
            }
        });
    }
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
        Ok(())=>format!("JVM lancee: {} entrypoint(s), ClientTickEvents.END_CLIENT_TICK actif (si enregistre)",entries.len()),
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
        assert!(valid_id("fabric-api"));
        assert!(!valid_id("../evil"));
        assert!(!valid_class("example::init"));
        assert!(!valid_class("../evil"));
    }
    #[test]
    fn rejects_unsupported_loader_features_and_unsafe_zip_before_java(){
        let root=std::env::temp_dir().join(format!("fabricbridge-metadata-test-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for (name,manifest,extra,expected) in [
            ("mixin",r#"{"id":"mixin","entrypoints":{"main":["x.Mod"]},"mixins":["x.mixins.json"]}"#,None,"mixins Fabric"),
            ("depends",r#"{"id":"depends","entrypoints":{"main":["x.Mod"]},"depends":{"fabric-api":"*"}}"#,None,"dependance non prise"),
            ("widener",r#"{"id":"widener","entrypoints":{"main":["x.Mod"]},"accessWidener":"some.accesswidener"}"#,None,"accessWidener"),
            ("nested",r#"{"id":"nested","entrypoints":{"main":["x.Mod"]},"jars":[{"file":"META-INF/jarjar/embedded.jar"}]}"#,None,"embarques"),
            ("traversal",r#"{"id":"traversal","entrypoints":{"main":["x.Mod"]}}"#,Some("../evil.class"),"nom de fichier JAR"),
        ] {
            let jar=root.join(format!("{name}.jar"));
            let mut zip=ZipWriter::new(File::create(&jar).unwrap());
            zip.start_file("fabric.mod.json",SimpleFileOptions::default()).unwrap();
            zip.write_all(manifest.as_bytes()).unwrap();
            if let Some(path)=extra {
                zip.start_file(path,SimpleFileOptions::default()).unwrap();
                zip.write_all(b"test").unwrap();
            }
            zip.finish().unwrap();
            let error=read_entrypoints(&jar).unwrap_err();
            assert!(error.contains(expected),"{name}: {error}");
        }
        let _=fs::remove_dir_all(root);
    }

    #[test]
    fn java_output_protocol_refuses_malformed_records(){
        assert_eq!(parse_command("HBR1\tfoo\tbar"),Some(("foo".into(),"bar".into())));
        assert!(parse_command("HBR1\tbad-name\tno").is_none());
        assert!(parse_command("HBR1\tfoo").is_none());
        assert!(parse_command("ordinary stdout line").is_none());
        assert!(parse_command(&format!("HBR1\tfoo\t{}", "x".repeat(721))).is_none());
    }
    #[test]
    fn rejects_invalid_entrypoint_instead_of_silently_skipping_it(){
        let root=std::env::temp_dir().join(format!("fabricbridge-strict-{}",std::process::id()));
        let _=fs::create_dir_all(&root);
        for (name,manifest,expected) in [
            ("adapter",r#"{"id":"strict","entrypoints":{"main":[{"adapter":"kotlin","value":"x.Mod"}]}}"#,"adapter"),
            ("custom",r#"{"id":"strict","entrypoints":{"custom":["x.Mod"]}}"#,"non pris en charge"),
            ("malformed",r#"{"id":"strict","entrypoints":{"main":[42]}}"#,"format invalide"),
            ("absent",r#"{"id":"strict","entrypoints":{"main":["x.Mod"]}}"#,"classe absente"),
            ("mixed",r#"{"id":"strict","entrypoints":{"main":["x.Mod",42]}}"#,"format invalide"),
            ("bad_environment",r#"{"id":"strict","environment":"server"}"#,"environment"),
            ("bad_depends",r#"{"id":"strict","depends":["missing"]}"#,"depends"),
        ] {
            let jar=root.join(format!("{name}.jar"));
            let mut zip=ZipWriter::new(File::create(&jar).unwrap());
            zip.start_file("fabric.mod.json",SimpleFileOptions::default()).unwrap();
            zip.write_all(manifest.as_bytes()).unwrap();
            if name=="mixed" {
                zip.start_file("x/Mod.class",SimpleFileOptions::default()).unwrap();
                zip.write_all(b"placeholder").unwrap();
            }
            zip.finish().unwrap();
            let error=read_entrypoints(&jar).unwrap_err();
            assert!(error.contains(expected),"{name}: {error}");
        }
        let _=fs::remove_dir_all(root);
    }
    #[test]
    fn bounded_jdk_output_does_not_deadlock_on_large_pipes(){
        if Command::new("java").arg("-version").output().is_err(){return;}
        let root=std::env::temp_dir().join(format!("fabricbridge-pipe-{}",std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source=root.join("PipeProof.java");
        fs::write(&source,r#"public class PipeProof {
    public static void main(String[] args) throws Exception {
        byte[] block=new byte[8192];
        for(int i=0;i<256;i++){
            System.out.write(block);
            System.err.write(block);
        }
    }
}"#).unwrap();
        let mut cmd=Command::new("java");
        cmd.arg(&source);
        let out=timed(cmd,Duration::from_secs(20)).unwrap();
        assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
        assert_eq!(out.stdout.len(),65536);
        assert_eq!(out.stderr.len(),65536);
        let _=fs::remove_dir_all(root);
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
    #[test]
    fn actual_client_tick_event_uses_live_horizon_position(){
        if Command::new("javac").arg("-version").output().is_err(){return;}
        let root=std::env::temp_dir().join(format!("fabricbridge-tick-test-{}",std::process::id()));
        let _=fs::remove_dir_all(&root);
        let src=root.join("src");let classes=root.join("classes");
        fs::create_dir_all(&classes).unwrap();
        let mut files=Vec::new();
        for (name,text) in shim_files().into_iter().chain(std::iter::once((
            "test/TickFabricMod.java",
            r#"package test;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import dev.horizon.fabricbridge.HorizonBridge;
public class TickFabricMod implements ClientModInitializer {
    private boolean once;
    public void onInitializeClient() {
        ClientTickEvents.END_CLIENT_TICK.register(client -> {
            if (!once && client.player != null) {
                once=true;
                HorizonBridge.registerCommand("javapos", "Position: "
                    +client.player.getBlockPos().getX()+","
                    +client.player.getBlockPos().getY()+","
                    +client.player.getBlockPos().getZ());
            }
        });
    }
}"#,
        ))){
            let p=src.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p,text).unwrap();files.push(p);
        }
        let compiled=Command::new("javac").arg("-d").arg(&classes).args(&files).output().unwrap();
        assert!(compiled.status.success(),"{}",String::from_utf8_lossy(&compiled.stderr));
        let jar=root.join("tick.jar");
        let mut zip=ZipWriter::new(File::create(&jar).unwrap());
        zip.start_file("fabric.mod.json",SimpleFileOptions::default()).unwrap();
        zip.write_all(br#"{"id":"tickproof","version":"1","entrypoints":{"client":["test.TickFabricMod"]}}"#).unwrap();
        zip.start_file("test/TickFabricMod.class",SimpleFileOptions::default()).unwrap();
        zip.write_all(&fs::read(classes.join("test/TickFabricMod.class")).unwrap()).unwrap();
        zip.finish().unwrap();
        let (id,entries)=read_entrypoints(&jar).unwrap();
        assert_eq!(id,"tickproof");
        assert_eq!(entries[0].0,"client");
        let separator=if cfg!(windows){';'}else{':'};
        let cp=format!("{}{}{}",classes.display(),separator,jar.display());
        let mut java=Command::new("java").arg("-cp").arg(cp)
            .arg("dev.horizon.fabricbridge.Runner").arg(&entries[0].0).arg(&entries[0].1)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        {
            let mut stdin=java.stdin.take().unwrap();
            writeln!(stdin,"NONE").unwrap();
            writeln!(stdin,"TICK\t10.9\t64.25\t-3.7").unwrap();
            // Close stdin: Java handles EOF and exits; unit test never blocks on a persistent JVM.
        }
        let out=java.wait_with_output().unwrap();
        assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
        assert!(String::from_utf8_lossy(&out.stdout).contains("HBR1\tjavapos\tPosition: 10,64,-4"),
            "{}",String::from_utf8_lossy(&out.stdout));
        let _=fs::remove_dir_all(root);
    }

}
