use crate::{configurate::ConfiguratePlugin, shop::ShopPlugin, Debug};
use bevy::{
    dev_tools::ui_debug_overlay::{DebugUiPlugin, UiDebugOptions},
    prelude::*,
};

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup_root)
            .add_systems(Update, sync_ui_outlines)
            .add_plugins(DebugUiPlugin)
            .add_plugins(ShopPlugin)
            .add_plugins(ConfiguratePlugin);
    }
}

#[derive(Resource)]
pub struct UIRoot(pub Entity);

fn sync_ui_outlines(mut debugui: ResMut<UiDebugOptions>, dbg: Res<Debug>) {
    debugui.enabled = dbg.gui_outline;
}

fn setup_root(mut cmd: Commands, mut debugui: ResMut<UiDebugOptions>) {
    debugui.enabled = true;

    let ui = cmd
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                align_content: AlignContent::Center,
                ..default()
            },
            ..default()
        })
        .id();

    cmd.insert_resource(UIRoot(ui));
}
