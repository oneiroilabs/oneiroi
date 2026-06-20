use bevy::{
    app::{App, Plugin, Update},
    ecs::{component::Component, query::With, resource::Resource, system::Query},
};
mod global;

pub struct Oneiroi;

impl Plugin for Oneiroi {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, oneiroi_main_loop);
    }
}

#[derive(Component)]
struct OneiroiInstance {}

fn oneiroi_main_loop(query: Query<&OneiroiInstance>) {
    println!("jaa");
    for thing in query {}
}
