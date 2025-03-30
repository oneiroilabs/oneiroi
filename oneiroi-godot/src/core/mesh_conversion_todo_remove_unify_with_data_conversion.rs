use std::cell::RefCell;

use godot::{
    classes::{
        ArrayMesh, Mesh, RenderingServer,
        mesh::{ArrayType, PrimitiveType},
    },
    prelude::*,
};
use oneiroi::mesh::{FaceHandle, OneiroiMesh};

//TODO this transformation should be a function which takes a OneiroiMesh or an type that can transform into a mesh
// and output the Godot Mesh
//Maybe even the Godot Counterpart
// OR have this sitting as a singleton smh and possible optimize away certain computations
// with a cache where we can lookup possibly through "path" references?
// in this case the Singleton has to b e the owner of the data/Meshes and gives out references
// only question that remains is how to deal with updates in that setup
// but that ties into the broader open question how editing graph changes get refleceted

#[derive(Clone, Debug)]
pub struct OneiroiMeshGD {
    mesh: RefCell<Option<Gd<Mesh>>>,
    oneiroi_mesh: OneiroiMesh,
    //TODO most likely operate directly on mesh with clear surface
    instance: RefCell<Option<Rid>>,
}

impl OneiroiMeshGD {
    pub fn new(oneiroi_mesh: OneiroiMesh) -> Self {
        let mesh = OneiroiMeshGD {
            mesh: RefCell::new(None),
            oneiroi_mesh,
            instance: RefCell::new(None),
        };

        mesh.update_mesh();

        mesh
    }

    /* pub fn get_instance(&self) -> Option<Rid> {
        *self.instance.borrow()
    } */

    /* pub fn get_Oneiroi_mesh(&self) -> &Option<OneiroiMesh> {
        self.Oneiroi_mesh.borrow()
    } */

    pub fn update_mesh(&self) {
        *self.mesh.borrow_mut() = Some(create_mesh(&self.oneiroi_mesh.clone()));

        let mut rendering_server = RenderingServer::singleton();

        *self.instance.borrow_mut() = Some(rendering_server.instance_create());

        rendering_server.instance_set_base(
            self.instance.borrow().expect("Has to be there"),
            self.mesh
                .borrow()
                .as_ref()
                .expect("Checked before its there")
                .get_rid(),
        );
    }

    pub fn get_instance(&self) -> Rid {
        //if not yet computed mesh from Internal Resource do it on first request
        //let mut mesh = self.mesh.borrow_mut();
        if self.mesh.borrow().is_none() {
            *self.mesh.borrow_mut() = Some(create_mesh(&self.oneiroi_mesh.clone()));
        }
        let mut rendering_server = RenderingServer::singleton();
        if self.instance.borrow().is_none() {
            *self.instance.borrow_mut() = Some(rendering_server.instance_create())
        }

        rendering_server.instance_set_base(
            self.instance.borrow().expect("Has to be there"),
            self.mesh
                .borrow()
                .as_ref()
                .expect("Checked before its there")
                .get_rid(),
        );
        self.instance.borrow().expect("Has to be there")
    }
}

///Creates the Godot Mesh
fn create_mesh(core_mesh: &OneiroiMesh) -> Gd<Mesh> {
    let mut mesh = ArrayMesh::new_gd();

    //maybe rename to access_surface
    let faces = core_mesh.get_faces();

    let mut surface = VariantArray::new();
    surface.resize(ArrayType::MAX.ord().try_into().unwrap(), &Variant::nil());

    let mut vertices: Vec<Vector3> = vec![];
    let mut normals: Vec<Vector3> = vec![];
    let mut indices: Vec<i32> = Vec::new();
    let points = core_mesh.get_points();

    // is is a piece of garbage and in the dirtiest way possible
    for (index, face) in faces.into_iter().enumerate() {
        for value in face.into_iter().rev() {
            //TODO this is a dirty hack and is not necessary

            vertices.push(Vector3::from_array(points[value.idx()].to_array()));
            //TODO this is highly unsafe
            indices.push(vertices.len() as i32 - 1);
            
            normals.push(Vector3::from_array(
                core_mesh
                    .get_face_normal(FaceHandle::new(index + 1))
                    .to_array(),
            ))
        }
    }

    surface.set(
        ArrayType::VERTEX.ord().try_into().unwrap(),
        &PackedVector3Array::from(vertices).to_variant(),
    );

    surface.set(
        ArrayType::NORMAL.ord().try_into().unwrap(),
        &PackedVector3Array::from(normals).to_variant(),
    );

    surface.set(
        ArrayType::INDEX.ord().try_into().unwrap(),
        &PackedInt32Array::from(indices).to_variant(),
    );
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &surface);

    mesh.upcast()
}
