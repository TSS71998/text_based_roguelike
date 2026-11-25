use crate::engine::{Ctx, LayerKind};
use crate::RunState;
use specs::prelude::*;

pub mod first_person;
pub mod top_down;

pub const MAP_LAYER: usize = 0;
pub const VIEW3D_LAYER: usize = 1;
pub const WORLD_LAYER: usize = 2;
pub const UI_LAYER: usize = 3;

pub const LAYERS: [LayerKind; 4] = [
    LayerKind::Simple,
    LayerKind::Pixels { x: VIEWPORT.0, y: VIEWPORT.1, w: VIEWPORT.2, h: VIEWPORT.3, width: 384, height: 352 },
    LayerKind::Fancy,
    LayerKind::Sparse
];

pub const VIEWPORT: (i32, i32, i32, i32) = (1, 1, 48, 44);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    TopDown,
    FirstPerson
}

impl ViewMode {
    pub fn toggled(self) -> ViewMode {
        match self {
            ViewMode::TopDown => ViewMode::FirstPerson,
            ViewMode::FirstPerson => ViewMode::TopDown
        }
    }
}

pub fn effective_view(ecs: &World) -> ViewMode {
    let chosen = ecs.try_fetch::<ViewMode>().map(|m| *m).unwrap_or_default();
    match ecs.try_fetch::<RunState>().map(|s| *s) {
        Some(RunState::ShowTargeting { .. }) => ViewMode::TopDown,
        _ => chosen
    }
}

pub fn clear_layers(ctx: &mut Ctx) {
    for layer in [MAP_LAYER, VIEW3D_LAYER, WORLD_LAYER, UI_LAYER] {
        ctx.set_active_console(layer);
        ctx.cls();
    }
    ctx.set_active_console(UI_LAYER);
}

pub fn render_world(ecs: &World, ctx: &mut Ctx) {
    match effective_view(ecs) {
        ViewMode::TopDown => {
            ctx.set_layer_visible(VIEW3D_LAYER, false);
            top_down::render(ecs, ctx);
        }
        ViewMode::FirstPerson => first_person::render(ecs, ctx)
    }
}