//! Latest page image for a vertex. Picking still uses the node transform.
//! While a frame is current the icosphere mesh is removed and a camera-facing
//! quad shows the page.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::render::SceneNodeKey;

pub const MAX_PAGE_EDGE: u32 = 640;
const PAGE_QUAD_WIDTH: f32 = 4.0;
const PAGE_QUAD_HEIGHT: f32 = 2.25;

#[derive(Clone, Debug)]
pub struct PageImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Resource, Default)]
pub struct PageFrames {
    pending: HashMap<String, PageImage>,
}

impl PageFrames {
    /// Keeps only the newest accepted image for `id`. Oversized or malformed
    /// frames are ignored so a bad capture cannot replace a good still.
    pub fn submit(&mut self, id: &str, width: u32, height: u32, rgba: Vec<u8>) -> bool {
        if !accepted(id, width, height, rgba.len()) {
            return false;
        }
        self.pending.insert(
            id.to_string(),
            PageImage {
                width,
                height,
                rgba,
            },
        );
        true
    }

    pub fn get(&self, id: &str) -> Option<&PageImage> {
        self.pending.get(id)
    }

    fn take_pending(&mut self) -> HashMap<String, PageImage> {
        std::mem::take(&mut self.pending)
    }
}

pub fn accepted(id: &str, width: u32, height: u32, rgba_len: usize) -> bool {
    !id.is_empty()
        && width > 0
        && height > 0
        && width <= MAX_PAGE_EDGE
        && height <= MAX_PAGE_EDGE
        && rgba_len == width as usize * height as usize * 4
}

#[derive(Component)]
pub(crate) struct PageQuad;

#[derive(Component)]
struct PageTexture(Handle<Image>);

#[derive(Resource)]
struct PageQuadMesh(Handle<Mesh>);

pub(crate) fn apply_page_frames(world: &mut World) {
    let pending = {
        let Some(mut frames) = world.get_resource_mut::<PageFrames>() else {
            return;
        };
        frames.take_pending()
    };
    if pending.is_empty() {
        return;
    }
    let nodes: HashMap<String, Entity> = world
        .query::<(Entity, &SceneNodeKey)>()
        .iter(world)
        .map(|(entity, key)| (key.0.clone(), entity))
        .collect();
    for (id, frame) in pending {
        let Some(entity) = nodes.get(&id).copied() else {
            continue;
        };
        show_frame(world, entity, frame);
    }
}

pub(crate) fn billboard_page_quads(
    camera: Query<&GlobalTransform, With<Camera>>,
    mut quads: Query<(&mut Transform, &GlobalTransform), With<PageQuad>>,
) {
    let Some(camera_transform) = camera.iter().next() else {
        return;
    };
    let camera_position = camera_transform.translation();
    for (mut transform, global) in &mut quads {
        let origin = global.translation();
        if origin.distance_squared(camera_position) < 1e-6 {
            continue;
        }
        // Parent rotation stays identity; local rotation is the world facing.
        transform.rotation = Transform::from_translation(origin)
            .looking_at(camera_position, Vec3::Y)
            .rotation;
    }
}

fn show_frame(world: &mut World, entity: Entity, frame: PageImage) {
    hide_sphere(world, entity);
    if let Some(quad) = find_quad(world, entity) {
        replace_texture(world, quad, frame);
        return;
    }
    let mesh = quad_mesh(world);
    let image = world.resource_mut::<Assets<Image>>().add(image_from(frame));
    let material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color_texture: Some(image.clone()),
            unlit: true,
            ..default()
        });
    let child = world
        .spawn((
            PageQuad,
            PageTexture(image),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
        ))
        .id();
    world.entity_mut(entity).add_child(child);
}

fn hide_sphere(world: &mut World, entity: Entity) {
    // Picking uses the node transform. Removing the mesh leaves that transform
    // in place and stops the icosphere from drawing over the page.
    world.entity_mut(entity).remove::<Mesh3d>();
    world
        .entity_mut(entity)
        .remove::<MeshMaterial3d<StandardMaterial>>();
}

fn find_quad(world: &World, entity: Entity) -> Option<Entity> {
    let children = world.get::<Children>(entity)?;
    children
        .iter()
        .find(|&child| world.get::<PageQuad>(child).is_some())
}

fn replace_texture(world: &mut World, quad: Entity, frame: PageImage) {
    let Some(handle) = world
        .get::<PageTexture>(quad)
        .map(|texture| texture.0.clone())
    else {
        return;
    };
    let mut images = world.resource_mut::<Assets<Image>>();
    let Some(image) = images.get_mut(&handle) else {
        return;
    };
    image.texture_descriptor.size = Extent3d {
        width: frame.width,
        height: frame.height,
        depth_or_array_layers: 1,
    };
    image.data = Some(frame.rgba);
}

fn quad_mesh(world: &mut World) -> Handle<Mesh> {
    if let Some(mesh) = world.get_resource::<PageQuadMesh>() {
        return mesh.0.clone();
    }
    let handle = world
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(PAGE_QUAD_WIDTH, PAGE_QUAD_HEIGHT));
    world.insert_resource(PageQuadMesh(handle.clone()));
    handle
}

fn image_from(frame: PageImage) -> Image {
    Image::new(
        Extent3d {
            width: frame.width,
            height: frame.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        frame.rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(value: u8) -> Vec<u8> {
        vec![value, 0, 0, 255]
    }

    fn app_with_node(id: &str) -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<PageFrames>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .add_systems(Update, apply_page_frames);
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(1.0, 1.0));
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let entity = app
            .world_mut()
            .spawn((
                SceneNodeKey(id.to_string()),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::default(),
            ))
            .id();
        (app, entity)
    }

    #[test]
    fn a_newer_frame_replaces_the_pending_image() {
        let mut frames = PageFrames::default();
        assert!(frames.submit("a", 1, 1, rgba(1)));
        assert!(frames.submit("a", 1, 1, rgba(2)));
        assert_eq!(frames.get("a").unwrap().rgba, rgba(2));
    }

    #[test]
    fn an_oversized_frame_is_ignored() {
        let mut frames = PageFrames::default();
        assert!(frames.submit("a", 1, 1, rgba(1)));
        let too_wide = vec![0; (MAX_PAGE_EDGE as usize + 1) * 4];
        assert!(!frames.submit("a", MAX_PAGE_EDGE + 1, 1, too_wide));
        assert_eq!(frames.get("a").unwrap().rgba, rgba(1));
    }

    #[test]
    fn an_unknown_id_does_not_create_a_quad() {
        let (mut app, _) = app_with_node("known");
        assert!(
            app.world_mut()
                .resource_mut::<PageFrames>()
                .submit("missing", 1, 1, rgba(9))
        );
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&PageQuad>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn a_known_vertex_hides_its_sphere_and_shows_the_latest_frame() {
        let (mut app, node) = app_with_node("known");
        {
            let mut frames = app.world_mut().resource_mut::<PageFrames>();
            assert!(frames.submit("known", 1, 1, rgba(1)));
            assert!(frames.submit("known", 1, 1, rgba(4)));
        }
        app.update();
        assert!(app.world().get::<Mesh3d>(node).is_none());
        let quad = app
            .world_mut()
            .query::<&PageQuad>()
            .iter(app.world())
            .count();
        assert_eq!(quad, 1);
        let handle = app
            .world_mut()
            .query::<&PageTexture>()
            .single(app.world())
            .unwrap()
            .0
            .clone();
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap();
        assert_eq!(image.data.as_deref(), Some(rgba(4).as_slice()));
    }

    #[test]
    fn removing_a_vertex_drops_its_frame() {
        let (mut app, node) = app_with_node("known");
        assert!(
            app.world_mut()
                .resource_mut::<PageFrames>()
                .submit("known", 1, 1, rgba(1))
        );
        app.update();
        app.world_mut().entity_mut(node).despawn();
        assert!(
            app.world_mut()
                .resource_mut::<PageFrames>()
                .submit("known", 1, 1, rgba(2))
        );
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&PageQuad>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
