//! Turns the state and settings into `render` shapes.
//!
//! Nutrition is one `render::set-corner` box (the game stacks it with its own
//! overlays). A mod has a single corner panel, so the armor and pickup
//! panels are placed by this module in GUI coordinates and sent together with
//! `render::set-hud`. Text and items cannot be faded or scaled by the API:
//! the scale setting applies to bars and icons, and notifications slide and
//! fade their background instead.

use crate::armor::Piece;
use crate::config::{ArmorConfig, Corner, NutritionConfig, PickupConfig};
use crate::nutrition::{self, MAX_FOOD, Snapshot};
use crate::pickup::Row;
use horizon_mod_sdk::render::{
    Corner as SdkCorner, FillShape, ItemShape, Point, Rect, Shape, TextShape,
};

const PAD: f32 = 3.0;
const LINE: f32 = 11.0;
const ICON: f32 = 16.0;
const GAP_ROWS: f32 = 2.0;
const MARGIN: f32 = 4.0;
const SEGMENTS: u32 = 10;

const BACKGROUND: u32 = 0x000000;
const TRACK: u32 = 0x3A3A3A;
const FILLED: u32 = 0xC8813A;
const GAIN: u32 = 0xFFE27A;
const TEXT: u32 = 0xFFFFFF;
const MUTED: u32 = 0xAAAAAA;
const WARN: u32 = 0xFF7777;

pub struct Panel {
    pub corner: SdkCorner,
    pub width: f32,
    pub height: f32,
    pub shapes: Vec<Shape>,
}

fn sdk_corner(corner: Corner) -> SdkCorner {
    match corner {
        Corner::TopLeft => SdkCorner::TopLeft,
        Corner::TopRight => SdkCorner::TopRight,
        Corner::BottomLeft => SdkCorner::BottomLeft,
        Corner::BottomRight => SdkCorner::BottomRight,
    }
}

/// 0xAARRGGBB from an opacity percentage and a weight (0 to 1).
fn argb(percent: u8, rgb: u32, weight: f32) -> u32 {
    let alpha = (255.0 * f32::from(percent) / 100.0 * weight).round() as u32;
    (alpha.min(255) << 24) | rgb
}

fn fill(x: f32, y: f32, w: f32, h: f32, color: u32) -> Shape {
    Shape::Fill(FillShape {
        rect: Rect { x, y, w, h },
        color,
    })
}

fn text(x: f32, y: f32, text: String, color: u32) -> Shape {
    Shape::Text(TextShape {
        text,
        at: Point { x, y },
        color,
        centered: false,
        shadow: true,
    })
}

fn translate(shape: &Shape, dx: f32, dy: f32) -> Shape {
    match shape {
        Shape::Fill(f) => fill(f.rect.x + dx, f.rect.y + dy, f.rect.w, f.rect.h, f.color),
        Shape::Text(t) => Shape::Text(TextShape {
            text: t.text.clone(),
            at: Point {
                x: t.at.x + dx,
                y: t.at.y + dy,
            },
            color: t.color,
            centered: t.centered,
            shadow: t.shadow,
        }),
        Shape::Item(i) => Shape::Item(ItemShape {
            item: i.item.clone(),
            at: Point {
                x: i.at.x + dx,
                y: i.at.y + dy,
            },
            scale: i.scale,
        }),
        other => other.clone(),
    }
}

/// The panel's shapes in GUI coordinates, in its corner of a `gui` screen.
pub fn place(panel: &Panel, gui: (f32, f32)) -> Vec<Shape> {
    let x = match panel.corner {
        SdkCorner::TopLeft | SdkCorner::BottomLeft => MARGIN,
        SdkCorner::TopRight | SdkCorner::BottomRight => (gui.0 - panel.width - MARGIN).max(0.0),
    };
    let y = match panel.corner {
        SdkCorner::TopLeft | SdkCorner::TopRight => MARGIN,
        SdkCorner::BottomLeft | SdkCorner::BottomRight => (gui.1 - panel.height - MARGIN).max(0.0),
    };
    panel.shapes.iter().map(|s| translate(s, x, y)).collect()
}

/// None when nothing should be drawn. `width_of` measures a line of text in
/// GUI pixels (`render::text_width` in the game).
pub fn nutrition_panel(
    cfg: &NutritionConfig,
    snap: &Snapshot,
    width_of: &dyn Fn(&str) -> f32,
) -> Option<Panel> {
    if !cfg.enabled {
        return None;
    }
    let held = if cfg.show_held {
        snap.held.as_ref()
    } else {
        None
    };
    if !cfg.show_bar && held.is_none() {
        return None;
    }

    let scale = cfg.scale_factor();
    let percent = cfg.opacity_percent();
    // The hunger bar preview works even if the food icon/row is hidden.
    let preview = snap
        .held
        .as_ref()
        .filter(|_| cfg.show_preview && cfg.show_bar)
        .map(|h| nutrition::preview(snap.food, h.points));
    let mut shapes: Vec<Shape> = Vec::new();
    let mut y = PAD;
    let mut inner_width: f32 = 0.0;

    if cfg.show_bar {
        let title = match preview {
            Some(p) if p.after > p.current => {
                format!("Faim {}/{} > {}", p.current, MAX_FOOD, p.after)
            }
            _ => format!("Faim {}/{}", snap.food, MAX_FOOD),
        };
        inner_width = inner_width.max(width_of(&title));
        shapes.push(text(PAD, y, title, TEXT));
        y += LINE;

        // Real values only: a line exists when the provider gave the number.
        if let Some(s) = snap.saturation {
            let line = format!("Saturation {s:.1}");
            inner_width = inner_width.max(width_of(&line));
            shapes.push(text(PAD, y, line, MUTED));
            y += LINE;
        }
        if let Some(e) = snap.exhaustion {
            let line = format!("Épuisement {e:.1}");
            inner_width = inner_width.max(width_of(&line));
            shapes.push(text(PAD, y, line, MUTED));
            y += LINE;
        }

        let seg = 8.0 * scale;
        let gap = scale;
        let after = preview.map_or(snap.food, |p| p.after);
        for i in 0..SEGMENTS {
            let x = PAD + i as f32 * (seg + gap);
            shapes.push(fill(x, y, seg, seg, argb(percent, TRACK, 1.0)));
            let have = nutrition::half_units(snap.food, i);
            let will = nutrition::half_units(after, i);
            let have_width = seg * have as f32 / 2.0;
            if have > 0 {
                shapes.push(fill(x, y, have_width, seg, argb(percent, FILLED, 1.0)));
            }
            if will > have {
                let gain_width = seg * (will - have) as f32 / 2.0;
                shapes.push(fill(
                    x + have_width,
                    y,
                    gain_width,
                    seg,
                    argb(percent, GAIN, 1.0),
                ));
            }
        }
        inner_width = inner_width.max(SEGMENTS as f32 * seg + (SEGMENTS - 1) as f32 * gap);
        y += seg + GAP_ROWS;
    }

    if let Some(h) = held {
        let icon = ICON * scale;
        shapes.push(Shape::Item(ItemShape {
            item: h.item.clone(),
            at: Point { x: PAD, y },
            scale,
        }));
        let name = nutrition::display_name(&h.item);
        let label = match h.saturation {
            Some(s) => format!("{name}: +{} faim, +{s:.1} sat.", h.points),
            None => format!("{name}: +{} faim", h.points),
        };
        inner_width = inner_width.max(icon + 4.0 + width_of(&label));
        shapes.push(text(PAD + icon + 4.0, y + (icon - 8.0) / 2.0, label, TEXT));
        y += icon + GAP_ROWS;
        if let Some(note) = h.note {
            inner_width = inner_width.max(width_of(note));
            shapes.push(text(PAD, y, note.to_string(), MUTED));
            y += LINE;
        }
        if let Some(p) = preview.filter(|p| p.wasted > 0) {
            let waste = format!("{} point(s) perdu(s)", p.wasted);
            inner_width = inner_width.max(width_of(&waste));
            shapes.push(text(PAD, y, waste, WARN));
            y += LINE;
        }
    }

    let width = inner_width + 2.0 * PAD;
    let height = y + PAD;
    shapes.insert(
        0,
        fill(0.0, 0.0, width, height, argb(percent, BACKGROUND, 0.55)),
    );
    Some(Panel {
        corner: sdk_corner(cfg.corner),
        width,
        height,
        shapes,
    })
}

/// Worn armor, head to feet: icon, a durability bar and the percentage in the
/// wear's color. Empty slots take no room.
pub fn armor_panel(
    cfg: &ArmorConfig,
    pieces: &[Option<Piece>],
    width_of: &dyn Fn(&str) -> f32,
) -> Option<Panel> {
    if !cfg.enabled {
        return None;
    }
    let scale = cfg.scale_factor();
    let icon = ICON * scale;
    let mut shapes: Vec<Shape> = Vec::new();
    let mut y = PAD;
    let mut inner_width: f32 = 0.0;
    let mut rows = 0;
    for piece in pieces.iter().flatten() {
        shapes.push(Shape::Item(ItemShape {
            item: piece.item.clone(),
            at: Point { x: PAD, y },
            scale,
        }));
        let mut row_width = icon;
        if let (Some(fraction), Some(wear), Some(pct)) =
            (piece.fraction(), piece.wear(), piece.percent())
        {
            let color = 0xFF00_0000 | wear.color();
            shapes.push(fill(PAD, y + icon - 2.0, icon, 2.0, 0xFF00_0000 | TRACK));
            shapes.push(fill(PAD, y + icon - 2.0, icon * fraction, 2.0, color));
            if cfg.show_percent {
                let label = format!("{pct}%");
                row_width += 3.0 + width_of(&label);
                shapes.push(text(
                    PAD + icon + 3.0,
                    y + (icon - 8.0) / 2.0,
                    label,
                    wear.color(),
                ));
            }
        }
        inner_width = inner_width.max(row_width);
        y += icon + GAP_ROWS;
        rows += 1;
    }
    if rows == 0 {
        return None;
    }
    let width = inner_width + 2.0 * PAD;
    let height = y + PAD;
    shapes.insert(
        0,
        fill(0.0, 0.0, width, height, argb(100, BACKGROUND, 0.45)),
    );
    Some(Panel {
        corner: sdk_corner(cfg.corner),
        width,
        height,
        shapes,
    })
}

/// The live pickup notifications, newest first, each sliding in from (and
/// back out to) the screen edge of its corner.
pub fn pickup_panel(cfg: &PickupConfig, rows: &[Row<'_>]) -> Option<Panel> {
    if !cfg.enabled || rows.is_empty() {
        return None;
    }
    let row_height = ICON + 2.0;
    let widths: Vec<f32> = rows
        .iter()
        .map(|r| PAD + ICON + 4.0 + r.label_w + PAD)
        .collect();
    let width = widths.iter().copied().fold(0.0_f32, f32::max);
    let right = matches!(cfg.corner, Corner::TopRight | Corner::BottomRight);
    let mut shapes: Vec<Shape> = Vec::new();
    for (i, (row, w)) in rows.iter().zip(&widths).enumerate() {
        let y = i as f32 * (row_height + GAP_ROWS);
        let away = (1.0 - row.visible) * (w + MARGIN * 2.0);
        let x = if right { width - w + away } else { -away };
        shapes.push(fill(
            x,
            y,
            *w,
            row_height,
            argb(100, BACKGROUND, 0.5 * row.visible),
        ));
        shapes.push(Shape::Item(ItemShape {
            item: row.item.to_string(),
            at: Point {
                x: x + PAD,
                y: y + 1.0,
            },
            scale: 1.0,
        }));
        shapes.push(text(
            x + PAD + ICON + 4.0,
            y + 1.0 + (ICON - 8.0) / 2.0,
            row.label.to_string(),
            TEXT,
        ));
    }
    let height = rows.len() as f32 * (row_height + GAP_ROWS);
    Some(Panel {
        corner: sdk_corner(cfg.corner),
        width,
        height,
        shapes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::armor;
    use crate::pickup::Notifier;
    use crate::provider::{FoodInfo, Stack, Vitals};

    fn width_of(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    fn snap(food: u32, held: Option<&str>) -> Snapshot {
        let vitals = Vitals {
            food,
            saturation: None,
            exhaustion: None,
        };
        let held = held.and_then(|id| {
            crate::food::lookup(id).map(|f| {
                let info = FoodInfo {
                    nutrition: f.points,
                    saturation: None,
                };
                (id, info)
            })
        });
        nutrition::snapshot(&vitals, held)
    }

    fn count(panel: &Panel, f: impl Fn(&Shape) -> bool) -> usize {
        panel.shapes.iter().filter(|s| f(s)).count()
    }

    #[test]
    fn nothing_is_drawn_when_disabled_or_empty() {
        let mut cfg = NutritionConfig::default();
        cfg.enabled = false;
        assert!(nutrition_panel(&cfg, &snap(10, None), &width_of).is_none());
        cfg.enabled = true;
        cfg.show_bar = false;
        assert!(nutrition_panel(&cfg, &snap(10, None), &width_of).is_none());
        let stone = snap(10, Some("minecraft:stone"));
        assert!(nutrition_panel(&cfg, &stone, &width_of).is_none());
        cfg.show_held = false;
        let apple = snap(10, Some("minecraft:apple"));
        assert!(nutrition_panel(&cfg, &apple, &width_of).is_none());
    }

    #[test]
    fn full_bar_without_food_in_hand() {
        let cfg = NutritionConfig::default();
        let panel = nutrition_panel(&cfg, &snap(20, None), &width_of).unwrap();
        // background + title + 10 tracks + 10 fills
        assert_eq!(panel.shapes.len(), 22);
        assert_eq!(count(&panel, |s| matches!(s, Shape::Item(_))), 0);
    }

    #[test]
    fn empty_bar_has_only_tracks() {
        let cfg = NutritionConfig::default();
        let panel = nutrition_panel(&cfg, &snap(0, None), &width_of).unwrap();
        assert_eq!(panel.shapes.len(), 12);
    }

    #[test]
    fn held_food_adds_icon_label_and_gain() {
        let cfg = NutritionConfig::default();
        let s = snap(14, Some("minecraft:apple"));
        let panel = nutrition_panel(&cfg, &s, &width_of).unwrap();
        // background + title + 10 tracks + 7 fills + 2 gains + icon + label
        assert_eq!(panel.shapes.len(), 23);
        assert_eq!(count(&panel, |s| matches!(s, Shape::Item(_))), 1);
    }

    #[test]
    fn preview_does_not_depend_on_held_row() {
        let mut cfg = NutritionConfig::default();
        cfg.show_held = false;
        let with_preview =
            nutrition_panel(&cfg, &snap(14, Some("minecraft:apple")), &width_of).unwrap();
        assert_eq!(count(&with_preview, |s| matches!(s, Shape::Item(_))), 0);
        assert_eq!(with_preview.shapes.len(), 21); // 10 tracks + 7 filled + 2 preview + background + title
        cfg.show_preview = false;
        let without = nutrition_panel(&cfg, &snap(14, Some("minecraft:apple")), &width_of).unwrap();
        assert_eq!(without.shapes.len(), 19);
    }

    #[test]
    fn preview_can_be_switched_off() {
        let mut cfg = NutritionConfig::default();
        cfg.show_preview = false;
        let s = snap(14, Some("minecraft:apple"));
        let panel = nutrition_panel(&cfg, &s, &width_of).unwrap();
        // background + title + 10 tracks + 7 fills + icon + label
        assert_eq!(panel.shapes.len(), 21);
    }

    #[test]
    fn saturation_and_exhaustion_lines_exist_only_when_given() {
        let cfg = NutritionConfig::default();
        let plain = snap(20, None);
        let mut real = plain.clone();
        real.saturation = Some(5.0);
        real.exhaustion = Some(1.2);
        let a = nutrition_panel(&cfg, &plain, &width_of).unwrap();
        let b = nutrition_panel(&cfg, &real, &width_of).unwrap();
        assert_eq!(b.shapes.len(), a.shapes.len() + 2);
        assert!(b.height > a.height);
    }

    #[test]
    fn wasted_points_and_notes_get_their_own_lines() {
        let cfg = NutritionConfig::default();
        let build = |food, item| nutrition_panel(&cfg, &snap(food, Some(item)), &width_of).unwrap();
        let plain = build(19, "minecraft:apple");
        let noted = build(19, "minecraft:pufferfish");
        let calm = build(10, "minecraft:apple");
        assert!(plain.height > calm.height);
        assert!(noted.height > calm.height);
    }

    fn stack(item: &str, damage: u32, max: u32) -> Option<Stack> {
        Some(Stack {
            item: item.into(),
            count: 1,
            damage,
            max_damage: max,
        })
    }

    #[test]
    fn armor_rows_hold_icon_bar_and_percentage() {
        let cfg = ArmorConfig::default();
        let equipment = vec![
            stack("minecraft:iron_helmet", 10, 100),
            stack("minecraft:iron_chestplate", 90, 100),
            None,
            stack("minecraft:iron_boots", 0, 100),
            None,
        ];
        let pieces = armor::pieces(&equipment);
        let panel = armor_panel(&cfg, &pieces, &width_of).unwrap();
        // background + 3 pieces x (icon + track + bar + percentage)
        assert_eq!(panel.shapes.len(), 13);
    }

    #[test]
    fn armor_percentage_can_be_hidden_and_empty_gear_draws_nothing() {
        let mut cfg = ArmorConfig::default();
        let pieces = armor::pieces(&[stack("minecraft:iron_helmet", 10, 100)]);
        cfg.show_percent = false;
        assert_eq!(
            armor_panel(&cfg, &pieces, &width_of).unwrap().shapes.len(),
            4
        );
        assert!(armor_panel(&cfg, &armor::pieces(&[None, None, None, None]), &width_of).is_none());
        cfg.enabled = false;
        assert!(armor_panel(&cfg, &pieces, &width_of).is_none());
    }

    #[test]
    fn pickup_rows_slide_from_their_edge() {
        let mut notifier = Notifier::new(5, 3.0);
        notifier.push("minecraft:stone", 2, &width_of);
        let rows = notifier.rows();
        let mut cfg = PickupConfig::default();
        cfg.corner = Corner::TopRight;
        let hidden = pickup_panel(&cfg, &rows).unwrap();
        notifier.advance(1.0);
        let shown = pickup_panel(&cfg, &notifier.rows()).unwrap();
        let x_of = |p: &Panel| match &p.shapes[0] {
            Shape::Fill(f) => f.rect.x,
            _ => f32::NAN,
        };
        // Right corner: sliding in moves the row from beyond the edge back.
        assert!(x_of(&hidden) > x_of(&shown));
        // background + icon + label per row
        assert_eq!(shown.shapes.len(), 3);
        cfg.enabled = false;
        assert!(pickup_panel(&cfg, &notifier.rows()).is_none());
    }

    #[test]
    fn panels_are_placed_in_their_corner() {
        let cfg = ArmorConfig::default();
        let pieces = armor::pieces(&[stack("minecraft:iron_helmet", 10, 100)]);
        let mut panel = armor_panel(&cfg, &pieces, &width_of).unwrap();
        let origin = |shapes: &[Shape]| match &shapes[0] {
            Shape::Fill(f) => (f.rect.x, f.rect.y),
            _ => (f32::NAN, f32::NAN),
        };
        panel.corner = SdkCorner::TopLeft;
        assert_eq!(origin(&place(&panel, (320.0, 180.0))), (MARGIN, MARGIN));
        panel.corner = SdkCorner::BottomRight;
        let (x, y) = origin(&place(&panel, (320.0, 180.0)));
        assert_eq!(x, 320.0 - panel.width - MARGIN);
        assert_eq!(y, 180.0 - panel.height - MARGIN);
    }
}
