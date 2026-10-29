// Status Effect Atelier: original card design with the game's own mob-effect
// atlas sprites (mob_effect/<id>). No Fabric/Cloth Config dependencies.
use super::{Game, Screen};
use horizon_client::{
    effects::EffectInstance,
    extras::Corner,
    hud::HudBatch,
    hud_layout,
    hud_overlays::{self, EffectIcon},
    inventory_effects,
};
use std::{cell::RefCell, collections::HashMap};

pub(super) const ID: &str = "status_effect_atelier";
const MAX_EFFECTS: usize = 16;
const KEYS: [(&str, &str, &str); 9] = [
    ("enabled", "Activer cartes", "true"),
    ("hud", "Affichage en jeu", "true"),
    ("inventory", "Affichage inventaire", "true"),
    ("mode", "Presentation", "cards"),
    ("side", "Position HUD", "left"),
    ("width", "Largeur", "medium"),
    ("max", "Nombre de cartes", "8"),
    ("ambient", "Effets ambiants", "true"),
    ("long", "Effets tres longs", "true"),
];
thread_local! {
    static MAX_DURATION: RefCell<HashMap<&'static str, i32>> = RefCell::new(HashMap::new());
}
fn value(game: &Game<'_>, key: &str, fallback: &str) -> String {
    game.native_setting(ID, key).unwrap_or_else(|| fallback.into())
}
fn on(game: &Game<'_>, key: &str, fallback: bool) -> bool {
    value(game,key,if fallback {"true"} else {"false"}) == "true"
}
fn enabled(game: &Game<'_>) -> bool {
    game.native_on(ID) && on(game,"enabled",true) && game.native_window()
}
pub(super) fn settings(game: &Game<'_>) -> Vec<String> {
    KEYS.iter().map(|(key, title, fallback)| {
        let v = value(game,key,fallback);
        let display = match (*key,v.as_str()) {
            (_, "true") => "ON",
            (_, "false") => "OFF",
            (_, "cards") => "Cartes",
            (_, "compact") => "Compact",
            (_, "icons") => "Icones",
            (_, "left") => "Gauche",
            (_, "right") => "Droite",
            (_, "small") => "Etroit",
            (_, "medium") => "Moyen",
            (_, "large") => "Large",
            _ => v.as_str(),
        };
        format!("{title}  [{display}]")
    }).collect()
}
pub(super) fn settings_click(game: &mut Game<'_>, index: usize) {
    let Some(&(key, _, fallback)) = KEYS.get(index) else {return;};
    let current = value(game,key,fallback);
    let next = match key {
        "mode" => match current.as_str() {"cards"=>"compact","compact"=>"icons",_=>"cards"},
        "side" => if current=="left" {"right"} else {"left"},
        "width" => match current.as_str() {"small"=>"medium","medium"=>"large",_=>"small"},
        "max" => match current.as_str() {"4"=>"6","6"=>"8","8"=>"12",_=>"4"},
        _ => if current=="true" {"false"} else {"true"},
    };
    game.native_set_setting(ID,key,next);
    if game.native_setting(ID,key).as_deref()!=Some(next) {
        game.native_chat("Status Effect Atelier : impossible de sauvegarder le reglage");
    }
}
fn format_duration(ticks: i32, rate: f32) -> String {
    if ticks == -1 {return "∞".into();}
    let hz = if rate.is_finite() && rate > 1.0 { rate } else {20.0};
    let secs = ((ticks.max(0) as f64)/(hz as f64)).ceil() as u64;
    if secs >= 3600 {format!("{}:{:02}:{:02}",secs/3600,(secs/60)%60,secs%60)}
    else {format!("{:02}:{:02}",secs/60,secs%60)}
}
fn roman(level: i32) -> String {
    const R: [&str; 10]=["I","II","III","IV","V","VI","VII","VIII","IX","X"];
    if (1..=10).contains(&level) {R[(level-1) as usize].into()} else {level.to_string()}
}
fn accent(icon: &EffectIcon) -> u32 {
    // Use the *actual effect color* from Horizon, brightened for contrast on
    // dark cards. Regen, speed, poison, fire resistance etc stay distinct.
    let color = icon.color as u32 & 0x00FF_FFFF;
    if color == 0 { return if icon.beneficial {0xFF58D6B1} else {0xFFF28A81}; }
    let brighten = |n: u32| (n * 3 / 4 + 58).min(255);
    let red = brighten((color >> 16) & 255);
    let green = brighten((color >> 8) & 255);
    let blue = brighten(color & 255);
    0xFF00_0000 | (red << 16) | (green << 8) | blue
}
fn clamp_ratio(duration: i32, max: i32) -> f32 {
    if duration == -1 {return 1.0;}
    if duration <= 0 || max <= 0 {return 0.0;}
    (duration as f32 / max as f32).clamp(0.0,1.0)
}
fn max_duration(id: &'static str, ticks: i32) -> i32 {
    if ticks <= 0 {return ticks;}
    MAX_DURATION.with(|cache|{
        let mut all=cache.borrow_mut();
        let best=all.entry(id).or_insert(ticks);
        if ticks > *best {*best=ticks;}
        *best
    })
}
fn reset_on_empty() { MAX_DURATION.with(|c| c.borrow_mut().clear()); }
struct Row<'a> {
    raw: &'a EffectInstance,
    icon: EffectIcon,
    title: String,
    time: String,
    ratio: f32,
}
fn rows(game: &Game<'_>) -> Vec<Row<'_>> {
    let all=&game.potions.effects.list;
    if all.is_empty() {reset_on_empty();return Vec::new();}
    // Cache only currently active effects, preventing stale maxima if one expires.
    MAX_DURATION.with(|c| c.borrow_mut().retain(|id,_|all.iter().any(|e|e.id==*id)));
    let max:usize=value(game,"max","8").parse::<usize>().unwrap_or(8).clamp(1,MAX_EFFECTS);
    let mut selected:Vec<_>=all.iter()
       .filter(|e|e.duration==-1 || e.duration>0)
       .filter(|e|on(game,"ambient",true)||!e.ambient)
       .filter(|e|on(game,"long",true)||e.duration==-1||e.duration<72000)
       .collect();
    selected.sort_by(|a,b|{
        let ia=EffectIcon::from(*a);
        let ib=EffectIcon::from(*b);
        ia.beneficial.cmp(&ib.beneficial).reverse()
          .then(a.duration.cmp(&b.duration))
          .then(a.id.cmp(b.id))
    });
    selected.into_iter().take(max).map(|e|{
        let icon=EffectIcon::from(e);
        let base=game.pack.names.display_get(&format!("effect.minecraft.{}",e.id))
            .unwrap_or_else(|| e.id.replace('_'," "));
        let title=if e.amplifier>0 {format!("{base} {}",roman(e.amplifier.saturating_add(1)))} else {base};
        let time=format_duration(e.duration,game.clock.rate());
        let ratio=clamp_ratio(e.duration,max_duration(e.id,e.duration));
        Row{raw:e,icon,title,time,ratio}
    }).collect()
}
fn fill(game:&Game<'_>,batch:&mut HudBatch,x:f32,y:f32,w:f32,h:f32,color:u32){
    if w>0.0 && h>0.0 {game.native_fill(batch,[x,y,w,h],color);}
}
fn clipped(game:&Game<'_>,name:&str,max_width:f32)->String{
    if max_width<=0.0 {return String::new();}
    if game.native_text_width(name)<=max_width {return name.to_string();}
    let mut s=name.to_string();
    while !s.is_empty() && game.native_text_width(&format!("{s}…"))>max_width {s.pop();}
    format!("{s}…")
}
fn icon(game:&Game<'_>,batch:&mut HudBatch,id:&str,x:f32,y:f32,small:bool) {
    let side=if small{17.0}else{20.0};
    fill(game,batch,x,y,side+6.0,side+6.0,0xE50E1421);
    fill(game,batch,x+1.0,y+1.0,side+4.0,side+4.0,0xFF34394A);
    let path=format!("mob_effect/{id}");
    if game.gui.atlas.sprites.contains_key(&path){
        batch.sprite_scaled(&game.gui.atlas,&path,x+3.0,y+3.0,side,side,u32::MAX);
    }else{
        game.native_text(batch,"?",[x+7.0,y+5.0],0xFFFFFFFF,true);
    }
}
fn card(game:&Game<'_>,batch:&mut HudBatch,row:&Row<'_>,x:f32,y:f32,w:f32,mode:&str) {
    let compact=mode=="compact";
    if mode=="icons" {
        let c=accent(&row.icon);
        fill(game,batch,x,y,29.0,29.0,c);
        icon(game,batch,row.raw.id,x+1.0,y+1.0,true);
        return;
    }
    let h=if compact{34.0}else{43.0};
    // Original nine-slice-inspired pixel geometry, drawn with HUD primitives.
    fill(game,batch,x+2.0,y+2.0,w,h,0x6905000C);
    fill(game,batch,x,y,w,h,0xE3121723);
    fill(game,batch,x,y,w,1.0,0xFF586073);
    fill(game,batch,x,y+h-1.0,w,1.0,0xFF070A10);
    let c=accent(&row.icon);
    fill(game,batch,x,y,3.0,h,c);
    fill(game,batch,x+3.0,y+1.0,w-4.0,1.0,0xFF323B4C);
    icon(game,batch,row.raw.id,x+7.0,y+5.0,compact);
    let textx=x+39.0;
    let max_name=(w-45.0).max(12.0);
    if compact {
        game.native_text(batch,&clipped(game,&row.title,max_name-45.0),[textx,y+7.0],0xFFF0F4FA,true);
        let tw=game.native_text_width(&row.time);
        game.native_text(batch,&row.time,[x+w-tw-7.0,y+7.0],0xFFC8C8D4,true);
    } else {
        game.native_text(batch,&clipped(game,&row.title,max_name),[textx,y+6.0],0xFFF0F4FA,true);
        game.native_text(batch,&row.time,[textx,y+19.0],0xFFABB4C8,true);
        let bonus=if row.raw.ambient {" •"} else {""};
        if !bonus.is_empty() {game.native_text(batch,bonus,[x+w-16.0,y+18.0],c,true);}
    }
    let track_x=textx;
    let track_w=(w-46.0).max(4.0);
    let by=y+h-7.0;
    fill(game,batch,track_x,by,track_w,3.0,0xFF414658);
    fill(game,batch,track_x,by,track_w*row.ratio,3.0,c);
    if row.raw.duration>=0 && row.raw.duration<=200 {
        fill(game,batch,x+w-3.0,y+1.0,2.0,h-2.0,0xFFFBBD68);
    }
}
fn vanilla_hud(game:&Game<'_>,batch:&mut HudBatch){
    let icons:Vec<_>=game.potions.effects.list.iter().map(EffectIcon::from).collect();
    let calls=hud_overlays::effects(batch.width as i32,false,&icons);
    hud_layout::paint(batch,&game.gui.atlas,&calls);
}
pub(super) fn hud(game:&Game<'_>,batch:&mut HudBatch) {
    draw_hud(game,batch,false);
}
fn draw_hud(game:&Game<'_>,batch:&mut HudBatch,force_inventory_fallback:bool) {
    let inventory_effects = matches!(game.screen, Screen::Inventory | Screen::Creative);
    if game.potions.effects.list.is_empty(){reset_on_empty();return;}
    if !enabled(game) {
        if !inventory_effects || force_inventory_fallback {vanilla_hud(game,batch);}
        return;
    }
    if !on(game,"hud",true) || (inventory_effects && !force_inventory_fallback) {return;}
    if !game.native_corner_shown(Corner::TopLeft) && value(game,"side","left")=="left" {return;}
    if !game.native_corner_shown(Corner::TopRight) && value(game,"side","left")=="right" {return;}
    let all=rows(game);
    let mode=value(game,"mode","cards");
    let w0=match value(game,"width","medium").as_str() {"small"=>138.0,"large"=>202.0,_=>170.0};
    let w=w0.min((batch.width-12.0).max(0.0));
    if w<30.0{return;}
    let right=value(game,"side","left")=="right";
    let x=if right{batch.width as f32-w-6.0}else{6.0};
    // Cap by GUI size and leave hotbar/chat space; do not paint off-screen.
    let stride=if mode=="icons"{32.0}else if mode=="compact"{37.0}else{46.0};
    let capacity=((batch.height as f32-48.0)/stride).floor().max(0.0) as usize;
    for (i,e) in all.iter().take(capacity).enumerate() {
        let dx=if mode=="icons"{i as f32*32.0}else{0.0};
        let dy=if mode=="icons"{0.0}else{i as f32*stride};
        // Icon-only mode uses horizontal strip; no out-of-screen panels.
        let x=if mode=="icons"{if right{batch.width as f32-6.0-(i+1) as f32*32.0}else{x+dx}}else{x};
        if x<2.0 || x>=batch.width as f32-24.0 {break;}
        card(game,batch,e,x,6.0+dy,w,&mode);
    }
}
pub(super) fn inventory(game:&Game<'_>,batch:&mut HudBatch,origin:(f32,f32),image_width:i32){
    let all=&game.potions.effects.list;
    if all.is_empty(){reset_on_empty();return;}
    if !enabled(game) {
        let panels=inventory_effects::panels(batch.width as i32,
            [origin.0 as i32,origin.1 as i32],image_width,
            all,game.clock.rate(),&|key|game.pack.names.display_get(key),
            &|text|game.native_text_width(text).ceil() as i32);
        let _=inventory_effects::paint(batch,&game.gui.atlas,&panels,
            [game.cursor.0 as i32,game.cursor.1 as i32]);
        return;
    }
    if !on(game,"inventory",true){return;}
    let x=origin.0+image_width as f32+4.0;
    let avail=batch.width as f32-x-5.0;
    if avail<32.0 { draw_hud(game,batch,true); return; }
    let w=match value(game,"width","medium").as_str(){
        "small"=>138.0,"large"=>202.0,_=>170.0
    }.min(avail);
    let mode=if w<96.0 {"icons".to_string()} else {value(game,"mode","cards")};
    let stride=if mode=="icons"{32.0}else if mode=="compact"{37.0}else{46.0};
    let list=rows(game);
    let capacity=((batch.height as f32-origin.1-5.0)/stride).floor().max(0.0) as usize;
    for (i,e) in list.iter().take(capacity).enumerate(){
        let y=origin.1+i as f32*stride;
        card(game,batch,e,x,y,w,&mode);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn progress_is_clamped_and_infinite(){
        assert_eq!(clamp_ratio(-1,20),1.0);
        assert_eq!(clamp_ratio(0,200),0.0);
        assert_eq!(clamp_ratio(400,100),1.0);
        assert!((clamp_ratio(45,90)-0.5).abs()<0.001);
    }
    #[test] fn duration_is_formatted_at_client_tick_rate(){
        assert_eq!(format_duration(-1,20.0),"∞");
        assert_eq!(format_duration(20,20.0),"00:01");
        assert_eq!(format_duration(1220,20.0),"01:01");
        assert_eq!(format_duration(72000,20.0),"1:00:00");
    }
    #[test] fn effect_palette_is_distinct_and_opaque(){
        let mut a = EffectIcon { id: "speed".into(), beneficial: true, color: 0x3366CC, ambient: false, duration: 200, show_icon: true };
        let blue = accent(&a);
        a.color = 0xCC3333;
        let red = accent(&a);
        assert_ne!(blue,red);
        assert_eq!(blue >> 24,0xFF);
        assert_eq!(red >> 24,0xFF);
    }
    #[test] fn roman_levels_are_readable(){
        assert_eq!(roman(2),"II");assert_eq!(roman(10),"X");assert_eq!(roman(11),"11");
    }
}
