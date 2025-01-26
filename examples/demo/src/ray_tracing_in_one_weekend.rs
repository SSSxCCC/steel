use glam::{vec3, Quat, Vec3};
use parry3d::shape::SharedShape;
use rand::{rngs::StdRng, Rng, SeedableRng};
use shipyard::{
    AllStoragesViewMut, Component, EntityId, IntoIter, IntoWithId, Remove, UniqueViewMut, ViewMut,
};
use steel::{
    data::Data,
    edit::Edit,
    hierarchy::{Children, Hierarchy, Parent},
    name::Name,
    render::{
        pipeline::raytracing::material::Material,
        renderer::{RenderObject, Renderer},
    },
    shape::Shape,
    transform::Transform,
};

/// Add this compoent to the scene, then the scene of RayTracingInOneWeekend will be generated under this entity.
#[derive(Component, Edit, Default)]
pub struct RayTracingInOneWeekend;

pub fn generate_scene_system(mut all_storage: AllStoragesViewMut) {
    let root = all_storage
        .run(|cs: ViewMut<RayTracingInOneWeekend>| cs.iter().with_id().next().map(|(e, _)| e));

    if let Some(root) = root {
        create_sphere(
            &mut all_storage,
            root,
            "Ground",
            vec3(0.0, -1000.0, 0.0),
            1000.0,
            vec3(0.5, 0.5, 0.5),
            Material::Lambertian,
        );

        create_sphere(
            &mut all_storage,
            root,
            "Big Sphere 0",
            vec3(0.0, 1.0, 0.0),
            1.0,
            vec3(0.5, 0.5, 0.5),
            Material::Dielectric { ri: 1.5 },
        );

        create_sphere(
            &mut all_storage,
            root,
            "Big Sphere 1",
            vec3(-4.0, 1.0, 0.0),
            1.0,
            vec3(0.4, 0.2, 0.1),
            Material::Lambertian,
        );

        create_sphere(
            &mut all_storage,
            root,
            "Big Sphere 2",
            vec3(4.0, 1.0, 0.0),
            1.0,
            vec3(0.7, 0.6, 0.5),
            Material::Metal { fuzz: 0.0 },
        );

        let mut rng = StdRng::from_entropy();
        for a in -11..11 {
            for b in -11..11 {
                let center = vec3(
                    a as f32 + 0.9 * rng.gen::<f32>(),
                    0.2,
                    b as f32 + 0.9 * rng.gen::<f32>(),
                );

                let choose_mat: f32 = rng.gen();

                if (center - vec3(4.0, 0.2, 0.0)).length() > 0.9 {
                    match choose_mat {
                        x if x < 0.8 => {
                            let albedo = vec3(rng.gen(), rng.gen(), rng.gen())
                                * vec3(rng.gen(), rng.gen(), rng.gen());
                            create_sphere(
                                &mut all_storage,
                                root,
                                format!("Sphere {a},{b}"),
                                center,
                                0.2,
                                albedo,
                                Material::Lambertian,
                            );
                        }
                        x if x < 0.95 => {
                            let albedo = vec3(
                                rng.gen_range(0.5..1.0),
                                rng.gen_range(0.5..1.0),
                                rng.gen_range(0.5..1.0),
                            );
                            let fuzz = rng.gen_range(0.0..0.5);
                            create_sphere(
                                &mut all_storage,
                                root,
                                format!("Sphere {a},{b}"),
                                center,
                                0.2,
                                albedo,
                                Material::Metal { fuzz },
                            );
                        }
                        _ => create_sphere(
                            &mut all_storage,
                            root,
                            format!("Sphere {a},{b}"),
                            center,
                            0.2,
                            Vec3::ONE,
                            Material::Dielectric { ri: 1.5 },
                        ),
                    }
                }
            }
        }

        all_storage.run(|mut cs: ViewMut<RayTracingInOneWeekend>| {
            cs.remove(root);
        });
    }
}

fn create_sphere(
    all_storage: &mut AllStoragesViewMut,
    parent: EntityId,
    name: impl Into<String>,
    position: Vec3,
    size: f32,
    color: Vec3,
    material: Material,
) {
    let e = all_storage.add_entity((
        Name::new(name),
        Transform {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(size),
        },
        Renderer {
            object: RenderObject::Shape(Shape(SharedShape::ball(1.0))),
            color: color.extend(1.0),
        },
        material,
    ));

    attach(all_storage, e, parent);
}

fn attach(all_storage: &AllStoragesViewMut, e: EntityId, parent: EntityId) {
    all_storage.run(
        |mut hierarchy: UniqueViewMut<Hierarchy>,
         mut childrens: ViewMut<Children>,
         mut parents: ViewMut<Parent>| {
            steel::hierarchy::attach_before(
                &mut hierarchy,
                &mut childrens,
                &mut parents,
                e,
                parent,
                EntityId::dead(),
            );
        },
    );
}
