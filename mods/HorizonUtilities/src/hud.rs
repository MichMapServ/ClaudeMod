//! Turns the nutrition state and settings into `render` shapes.
//!
//! The panel is one `render::set-corner` box (the game stacks it with its own
//! overlays), sent only when something changed. Text cannot be scaled by the
//! API, so the scale setting applies to the bar and the item icon.

use crate::config::{Corner, NutritionConfig};
use crate::nutrition::{self, MAX_FOOD, Snapshot};
use horizon_mod_sdk::render::{
    Corner as SdkCorner, FillShape, ItemShape, Point, Rect, Shape, TextShape,
};

const PAD: f32 = 3.0;
const LINE: f32 = 11.0;
const ICON: f32 = 16.0;
const GAP_ROWS: f32 = 2.0;
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

/// None when nothing should be drawn. `width_of` measures a line of text in
/// GUI pixels (`render::text_width` in the game).
pub fn build(
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
    let preview = held
        .filter(|_| cfg.show_preview)
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
        let label = format!("{}: +{} faim", nutrition::display_name(&h.item), h.points);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn width_of(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    fn snap(food: u32, held: Option<&str>) -> Snapshot {
        nutrition::snapshot(food, held)
    }

    fn count(panel: &Panel, f: impl Fn(&Shape) -> bool) -> usize {
        panel.shapes.iter().filter(|s| f(s)).count()
    }

    #[test]
    fn nothing_is_drawn_when_disabled_or_empty() {
        let mut cfg = NutritionConfig::default();
        cfg.enabled = false;
        assert!(build(&cfg, &snap(10, None), &width_of).is_none());
        cfg.enabled = true;
        cfg.show_bar = false;
        assert!(build(&cfg, &snap(10, None), &width_of).is_none());
        assert!(build(&cfg, &snap(10, Some("minecraft:stone")), &width_of).is_none());
        cfg.show_held = false;
        assert!(build(&cfg, &snap(10, Some("minecraft:apple")), &width_of).is_none());
    }

    #[test]
    fn full_bar_without_food_in_hand() {
        let cfg = NutritionConfig::default();
        let panel = build(&cfg, &snap(20, None), &width_of).unwrap();
        // background + title + 10 tracks + 10 fills
        assert_eq!(panel.shapes.len(), 22);
        assert_eq!(count(&panel, |s| matches!(s, Shape::Item(_))), 0);
    }

    #[test]
    fn empty_bar_has_only_tracks() {
        let cfg = NutritionConfig::default();
        let panel = build(&cfg, &snap(0, None), &width_of).unwrap();
        assert_eq!(panel.shapes.len(), 12);
    }

    #[test]
    fn held_food_adds_icon_label_and_gain() {
        let cfg = NutritionConfig::default();
        let panel = build(&cfg, &snap(14, Some("minecraft:apple")), &width_of).unwrap();
        // background + title + 10 tracks + 7 fills + 2 gains + icon + label
        assert_eq!(panel.shapes.len(), 23);
        assert_eq!(count(&panel, |s| matches!(s, Shape::Item(_))), 1);
    }

    #[test]
    fn preview_can_be_switched_off() {
        let mut cfg = NutritionConfig::default();
        cfg.show_preview = false;
        let panel = build(&cfg, &snap(14, Some("minecraft:apple")), &width_of).unwrap();
        // no gain segments: background + title + 10 tracks + 7 fills + icon + label
        assert_eq!(panel.shapes.len(), 21);
    }

    #[test]
    fn wasted_points_and_notes_get_their_own_lines() {
        let cfg = NutritionConfig::default();
        let plain = build(&cfg, &snap(19, Some("minecraft:apple")), &width_of).unwrap();
        let noted = build(&cfg, &snap(19, Some("minecraft:pufferfish")), &width_of).unwrap();
        let calm = build(&cfg, &snap(10, Some("minecraft:apple")), &width_of).unwrap();
        assert!(plain.height > calm.height);
        assert!(noted.height > calm.height);
    }

    #[test]
    fn the_box_fits_its_content() {
        let cfg = NutritionConfig::default();
        let panel = build(&cfg, &snap(10, Some("minecraft:cooked_beef")), &width_of).unwrap();
        assert!(panel.width >= 89.0 + 2.0 * PAD);
        assert!(panel.height > 2.0 * PAD);
    }
}
