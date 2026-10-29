// HorizonUtilities Native v0.5.0. Shared implementation injected into game.rs.
// No process patching, memory scanning, network access or untrusted external code.
use super::{Game, Screen};
use horizon_client::extras::Corner;
use horizon_client::extras_hud::Corners;
use horizon_client::hud::HudBatch;
use horizon_client::inv_sort;
use horizon_client::items::ItemStack;
use horizon_client::menu::Input;
use std::cell::RefCell;
use std::time::{Duration, Instant};

pub const ID: &str = "horizon_utilities";
pub const VERSION: &str = "0.5.0";
const STORAGE: &str = "waypoints_native.v2";
const LEGACY: &str = "waypoints_native.v1";
const ACTIVE_PROFILE: &str = "profile_native.v2";
const MAX_PROFILES: usize = 12;
const MAX_POINTS: usize = 8;

thread_local! {
    static RECENT_PICKUP: RefCell<Option<(String, Instant)>> = const { RefCell::new(None) };
}
const OPTIONS: [(&str, &str, bool); 9] = [
    ("enabled", "HUD principal", true),
    ("nutrition", "Faim et saturation", true),
    ("armor", "Armure et durabilite", true),
    ("pickup", "Ramassages confirmes", true),
    ("performance", "FPS et CPS", true),
    ("inventory", "Occupation de l'inventaire", false),
    ("navigation", "Navigation", true),
    ("minimap", "Mini-carte", false),
    ("compact", "Affichage compact", true),
];
fn option(game: &Game<'_>, name: &str, fallback: bool) -> bool {
    match game.native_setting(ID, name).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => fallback,
    }
}
fn active(game: &Game<'_>, key: &str, fallback: bool) -> bool {
    game.native_on(ID) && option(game, "enabled", true) && option(game, key, fallback)
}
pub(super) fn settings(game: &Game<'_>) -> Vec<String> {
    OPTIONS.iter().map(|(key, title, fallback)| {
        format!("{title}  [{}]", if option(game, key, *fallback) {"ON"} else {"OFF"})
    }).collect()
}
pub(super) fn settings_click(game: &mut Game<'_>, index: usize) {
    if let Some(&(key, _, fallback)) = OPTIONS.get(index) {
        let next = !option(game, key, fallback);
        game.native_set_setting(ID, key, if next {"true"} else {"false"});
    }
}
pub(super) fn key(game: &mut Game<'_>, key: &str) {
    if game.native_on(ID) && game.native_playing()
        && key == game.native_mod_key(ID, "toggle", "key.keyboard.n") {
        let next = !option(game, "enabled", true);
        game.native_set_setting(ID, "enabled", if next {"true"} else {"false"});
    }
}
pub(super) fn minimap(game: &Game<'_>, out: &mut Option<crate::game::native_hooks::NativeMinimap>) {
    if out.is_some() || !active(game, "minimap", false) || game.native_player().is_none() { return; }
    *out = Some(crate::game::native_hooks::NativeMinimap {
        corner: Corner::TopRight, size: 112.0, blocks_per_pixel: 2.0, rotate: true, round: true,
    });
}
/// The client reports accepted item-take animation events. The packet does not
/// supply the accepted quantity of a partial take; never invent that number.
pub(super) fn pickup(id: &str) {
    let name = id.strip_prefix("minecraft:").unwrap_or(id).replace('_', " ");
    RECENT_PICKUP.with(|state| *state.borrow_mut() = Some((name, Instant::now())));
}

#[derive(Clone, Debug, PartialEq)]
struct Waypoint { profile: String, name: String, dimension: String, x: f64, y: f64, z: f64 }
fn valid_ident(s: &str) -> bool {
    !s.is_empty() && s.len() <= 24 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn finite_pos(x: f64, y: f64, z: f64) -> bool {
    [x,y,z].iter().all(|n| n.is_finite() && n.abs() <= 30_000_000.0)
}
fn parse_point(line: &str) -> Option<Waypoint> {
    let parts: Vec<_> = line.split('|').collect();
    let [profile,name,dimension,x,y,z] = parts.as_slice() else { return None };
    if !valid_ident(profile) || !valid_ident(name)
        || dimension.is_empty() || dimension.len() > 128 || !dimension.is_ascii() || dimension.contains('|') {
        return None;
    }
    let (x,y,z) = (x.parse().ok()?, y.parse().ok()?, z.parse().ok()?);
    if !finite_pos(x,y,z) { return None; }
    Some(Waypoint { profile: (*profile).into(), name: (*name).into(), dimension: (*dimension).into(), x,y,z })
}
fn get_points(game: &Game<'_>) -> Vec<Waypoint> {
    // v1 is imported as an inert "legacy" profile, NEVER auto-selected.
    let raw = match game.native_setting(ID, STORAGE) {
        Some(v) => v,
        None => game.native_setting(ID, LEGACY).unwrap_or_default()
            .lines().filter_map(|line| parse_point(&format!("legacy|{line}")))
            .map(|p| format!("{}|{}|{}|{}|{}|{}", p.profile,p.name,p.dimension,p.x,p.y,p.z))
            .collect::<Vec<_>>().join("\n"),
    };
    if raw.len() > 16384 { return Vec::new(); }
    let mut out = Vec::new();
    for line in raw.lines() {
        if let Some(p) = parse_point(line) {
            if !out.iter().any(|w: &Waypoint| w.profile == p.profile && w.name == p.name)
                && out.iter().filter(|w: &&Waypoint| w.profile == p.profile).count() < MAX_POINTS
                && (out.iter().any(|w: &Waypoint| w.profile == p.profile) ||
                    out.iter().map(|w: &Waypoint| &w.profile).collect::<std::collections::HashSet<_>>().len() < MAX_PROFILES) {
                out.push(p);
            }
        }
    }
    out
}
fn save_points(game: &Game<'_>, points: &[Waypoint]) -> Result<(), String> {
    let raw = points.iter()
        .map(|p| format!("{}|{}|{}|{:.3}|{:.3}|{:.3}",p.profile,p.name,p.dimension,p.x,p.y,p.z))
        .collect::<Vec<_>>().join("\n");
    if raw.len() > 16384 { return Err("Stockage plein".into()); }
    game.native_set_setting(ID, STORAGE, &raw);
    Ok(())
}
fn profile(game: &Game<'_>) -> Option<String> {
    game.native_setting(ID, ACTIVE_PROFILE).filter(|p| valid_ident(p))
}
fn current_point(game: &Game<'_>) -> Option<Waypoint> {
    let profile = profile(game)?;
    let name = game.native_setting(ID,"selected_waypoint")?;
    get_points(game).into_iter().find(|p| p.profile == profile && p.name == name)
}
fn label(item: &str) -> &str { item.strip_prefix("minecraft:").unwrap_or(item) }
fn panel(game: &Game<'_>, batch: &mut HudBatch, corners: &mut Corners, corner: Corner, rows: &[(String,u32)]) {
    if rows.is_empty() || !game.native_corner_shown(corner) { return; }
    let width = rows.iter().map(|(s,_)| game.native_text_width(s)).fold(0.0f32,f32::max)+14.0;
    let height = 9.0 + 11.0 * rows.len() as f32;
    let (x,y) = corners.place(corner,width,height);
    game.native_fill(batch,[x,y,width,height],0xB3151D2D);
    // Accent line, intended to be visible on both bright and dark terrain.
    game.native_fill(batch,[x,y,2.0,height],0xFF56D6C6);
    for (i,(s,color)) in rows.iter().enumerate() {
        game.native_text(batch,s,[x+7.0,y+4.0+i as f32*11.0],*color,true);
    }
}
pub(super) fn draw(game: &mut Game<'_>, batch: &mut HudBatch, corners: &mut Corners) {
    if !game.native_window() || game.player.is_none() || !active(game,"enabled",true) { return; }
    let compact=option(game,"compact",true);
    let mut vital:Vec<(String,u32)>=Vec::new();
    let mut extra:Vec<(String,u32)>=Vec::new();
    if option(game,"nutrition",true) {
        if let Some(p)=game.player.as_ref() {
            let f=p.health.food.food;
            let s=p.health.food.saturation;
            vital.push((format!("NUTRITION  {f}/20  SAT {s:.1}"),0xFF6AE2D2));
            if f<=6 || p.health.health()<=6.0 {
                vital.push((format!("! Survie  vie {:.1}  faim {f}",p.health.health()),0xFFFF8F8F));
            }
            if !compact {
                if let Some(stack)=game.inventory.get(game.selected).and_then(Option::as_ref) {
                    if let Some((points,_))=game.res.items().get(&stack.item).and_then(|i| i.food) {
                        let after=(f+points as i32).min(20);
                        vital.push((format!("{} : faim {f} -> {after}",label(&stack.item)),0xFFD4E3FF));
                    }
                }
            }
        }
    }
    if option(game,"armor",true) {
        let names=["Tete","Torse","Jambes","Bottes"];
        for (i,title) in names.iter().enumerate() {
            if let Some(item)=game.armor[i].as_ref() {
                let max=game.res.items().get(&item.item).map_or(0,|it| it.max_damage);
                let color=if max>0 && item.damage.saturating_mul(4)>=max.saturating_mul(3) {0xFFFF9A8B} else {0xFFCDD8F4};
                let info=if max>0 {
                    format!("{}% restant",max.saturating_sub(item.damage).saturating_mul(100)/max)
                } else { "durabilite N/A".into() };
                let title=if compact { title.to_string() } else {format!("{title} {}",label(&item.item))};
                vital.push((format!("{title}: {info}"),color));
            } else if !compact { vital.push((format!("{title}: vide"),0xFF8797AE)); }
        }
    }
    if option(game,"performance",true) {
        let (fps,_,low)=game.native_frame_rate();
        let (cps,use_cps)=game.native_clicks();
        extra.push((format!("FPS {fps}  1% {low}  CPS {cps}/{use_cps}"),0xFF8FE6FF));
    }
    if option(game,"inventory",false) {
        let used=game.inventory.iter().filter(|s|s.is_some()).count();
        extra.push((format!("Inventaire {used}/{}",game.inventory.len()),0xFFCDD8F4));
    }
    if option(game,"navigation",true) {
        if let (Some(p),Some(w))=(game.native_player(),current_point(game)) {
            if p.dimension==w.dimension {
                let dx=w.x-p.x;let dy=w.y-p.y;let dz=w.z-p.z;
                extra.push((format!("{}: {:.0}m  Y {dy:+.0}",w.name,(dx*dx+dy*dy+dz*dz).sqrt()),0xFFFFD88A));
            } else {extra.push((format!("{}: autre dimension",w.name),0xFFFFD88A));}
        }
    }
    if option(game,"pickup",true) {
        RECENT_PICKUP.with(|state| {
            if let Some((id,at))=state.borrow().as_ref() {
                if at.elapsed() < Duration::from_secs(3) {
                    extra.push((format!("Ramasse: {id} (quantite inconnue)"),0xFF9EE7AC));
                }
            }
        });
    }
    panel(game,batch,corners,Corner::BottomRight,&vital);
    panel(game,batch,corners,Corner::TopRight,&extra);
}
fn sort(game:&mut Game<'_>,side:&str)->Result<usize,String>{
    if game.screen!=Screen::Inventory || game.carried.is_some() || game.station.is_some() || game.mount_active(){
        return Err("Ouvrir un inventaire avec le curseur vide".into());
    }
    let slots:Vec<usize>=match side {
        "main"=>(9..36).collect(),
        "chest"=>{
            if game.remote.is_some() {
                if !game.remote_storage_open(){return Err("Conteneur distant indisponible".into())}
            }else if !matches!(game.container_kind(),Some(horizon_client::containers::BlockEntity::Chest(_))){
                return Err("Ouvrir un coffre compatible".into());
            }
            game.slots().into_iter().filter(|s|(300..500).contains(s)).collect()
        }
        _=>return Err("Utiliser sort main ou sort chest".into()),
    };
    if slots.is_empty() || slots.len()>216 {return Err("Zone de tri invalide".into())}
    let view:Vec<Option<ItemStack>>=slots.iter().map(|&i|game.slot_view(i)).collect();
    let plan={
        let rank=|s:&ItemStack|(0usize,s.item.to_string());
        let max=|s:&ItemStack|game.res.items().get(&s.item).map_or(64,|i|i.max_stack).max(1);
        inv_sort::plan(&view,&rank,&max)
    };
    if plan.len()>512{return Err("Tri trop complexe".into())}
    // Never mutate raw slots: use the game's own validated menu-click path.
    let mut done=0;
    for index in plan {
        if !game.tweak_click(slots[index],0,Input::Pickup) {
            return Err(format!("Tri interrompu apres {done} clics, etat a verifier"));
        }
        done+=1;
    }
    Ok(done)
}
pub(super) fn command(game:&mut Game<'_>,text:&str) {
    if !game.native_on(ID) {return}
    let args:Vec<_>=text.split_whitespace().collect();
    let result:Result<String,String>=match args.as_slice() {
        ["utilities"]|["utilities","help"]=>Ok("/utilities status | toggle | sort main|chest | profile use|off|list NOM | wp add|select|remove|list|off NOM".into()),
        ["utilities","status"]=>Ok(format!("HorizonUtilities {VERSION} | profil {}",profile(game).unwrap_or_else(||"AUCUN".into()))),
        ["utilities","toggle"]=>{
            let enabled=!option(game,"enabled",true);
            game.native_set_setting(ID,"enabled",if enabled {"true"}else{"false"});
            Ok(format!("HorizonUtilities {}",if enabled {"ON"}else{"OFF"}))
        },
        ["utilities","sort",side]=>sort(game,side).map(|n|format!("Tri: {n} clics")),
        ["utilities","profile","use",name] if valid_ident(name)=>{
            game.native_set_setting(ID,ACTIVE_PROFILE,name);
            game.native_set_setting(ID,"selected_waypoint","");
            Ok(format!("Profil {name} actif (associe-le uniquement au bon monde)"))
        },
        ["utilities","profile","off"]=>{
            game.native_set_setting(ID,ACTIVE_PROFILE,"");
            game.native_set_setting(ID,"selected_waypoint","");
            Ok("Profil desactive".into())
        },
        ["utilities","profile","list"]=>{
            let mut names:Vec<_>=get_points(game).iter().map(|p|p.profile.clone()).collect();
            names.sort();names.dedup();
            Ok(if names.is_empty(){"Aucun profil".into()}else{names.join(", ")})
        },
        ["utilities","wp","list"]=>{
            let current=profile(game);
            let names:Vec<_>=get_points(game).into_iter().filter(|p|Some(p.profile.clone())==current).map(|p|p.name).collect();
            Ok(if names.is_empty(){"Aucun waypoint dans le profil".into()}else{names.join(", ")})
        },
        ["utilities","wp","add",name] if valid_ident(name)=>{
            if let (Some(profile),Some(player))=(profile(game),game.native_player()){
                let mut points=get_points(game);
                if let Some(p)=points.iter_mut().find(|p|p.profile==profile && p.name==*name) {
                    p.dimension=player.dimension.into();p.x=player.x;p.y=player.y;p.z=player.z;
                } else {
                    if points.iter().filter(|p|p.profile==profile).count()>=MAX_POINTS {game.native_chat("Maximum 8 points par profil");return;}
                    if !points.iter().any(|p|p.profile==profile)
                        && points.iter().map(|p|&p.profile).collect::<std::collections::HashSet<_>>().len()>=MAX_PROFILES {
                        game.native_chat("Maximum 12 profils");return;
                    }
                    points.push(Waypoint{profile:profile.clone(),name:(*name).into(),dimension:player.dimension.into(),x:player.x,y:player.y,z:player.z});
                }
                save_points(game,&points).map(|()|{
                    game.native_set_setting(ID,"selected_waypoint",name);
                    format!("Point {name} sauvegarde dans {profile}")
                })
            }else{Err("Choisir /utilities profile use NOM et entrer dans un monde".into())}
        },
        ["utilities","wp","select",name] if valid_ident(name)=>{
            if get_points(game).iter().any(|p|Some(&p.profile)==profile(game).as_ref() && p.name==*name) {
                game.native_set_setting(ID,"selected_waypoint",name);
                Ok(format!("Navigation vers {name}"))
            }else{Err("Waypoint absent du profil".into())}
        },
        ["utilities","wp","remove",name] if valid_ident(name)=>{
            let selected=profile(game);
            let mut points=get_points(game);
            let before=points.len();
            points.retain(|p|Some(&p.profile)!=selected.as_ref() || p.name!=*name);
            if points.len()==before {Err("Waypoint introuvable".into())}else{
                save_points(game,&points).map(|()|{
                    if game.native_setting(ID,"selected_waypoint").as_deref()==Some(name) {
                        game.native_set_setting(ID,"selected_waypoint","");
                    }
                    format!("{name} supprime")
                })
            }
        },
        ["utilities","wp","off"]=>{game.native_set_setting(ID,"selected_waypoint","");Ok("Navigation desactivee".into())},
        _=>Err("Commande invalide: /utilities help".into())
    };
    game.native_chat(&result.unwrap_or_else(|error|format!("HorizonUtilities: {error}")));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn identifiers_are_bounded(){assert!(valid_ident("base_1"));assert!(!valid_ident("../x"));assert!(!valid_ident(""));assert!(!valid_ident(&"x".repeat(25)));}
    #[test] fn coordinates_are_finite(){assert!(!finite_pos(f64::NAN,0.0,0.0));assert!(!finite_pos(0.0,f64::INFINITY,0.0));assert!(finite_pos(10.0,64.0,-10.0));}
    #[test] fn corrupted_waypoints_are_ignored(){assert!(parse_point("a|b|minecraft:overworld|NaN|4|5").is_none());assert!(parse_point("a|b|minecraft:overworld|1|2|3").is_some());}
}
