//! The uniforms of the draw shaders, as the backends which bind them
//! lay them out.



/// The uniform of a [`DrawShader`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawArgs {
    /// Column-major, as WGSL wants it.
    pub view_projection: [[f32; 4]; 4],
    /// Points from what is drawn towards the light.
    pub light: [f32; 4],
    pub color: [f32; 4],
    /// The word the first element starts at in the page.
    pub offset: u32,
    /// Steps along every element.
    pub rings: u32,
    /// Steps across every patch of a surface.
    pub sides: u32,
    pub _padding: u32,
}

/// The uniform of a draw shader of instances, at binding 5.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceArgs {
    pub rows_offset: u32,
    pub rows: u32,
    pub groups_offset: u32,
    pub groups: u32,
    /// How many elements the shape has.
    pub elements: u32,
    /// How many elements its longest group has.
    pub slots: u32,
    pub _padding: [u32; 2],
}

/// The uniform of [`cull`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CullArgs {
    /// Column-major, as WGSL wants it.
    pub view_projection: [[f32; 4]; 4],
    /// The size in pixels of what is drawn into.
    pub viewport: [f32; 2],
    /// Instances smaller than this many pixels across are not drawn.
    pub min_pixels: f32,
    /// The word the first row starts at in its page, and how many there are.
    pub rows_offset: u32,
    pub rows: u32,
    /// How many places for pieces each visible instance gets: see
    /// [`InstanceArgs::slots`].
    pub slots: u32,
    /// How many vertices a draw without mesh shaders has per piece.
    pub vertices: u32,
    pub _padding: u32,
}

/// Where [`cull`] writes the arguments of what comes after it, in words of
/// the indirect buffer: task shaders, a draw without them, and counting.
pub const CULL_TASKS: u64 = 0;

pub const CULL_DRAW: u64 = 4;

pub const CULL_COUNT: u64 = 8;

/// How long the indirect buffer is, in words.
pub const CULL_WORDS: u64 = 12;

/// The uniform of a [`LodShader`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LodArgs {
    /// Column-major, as WGSL wants it.
    pub view_projection: [[f32; 4]; 4],
    /// Points from what is drawn towards the light.
    pub light: [f32; 4],
    pub color: [f32; 4],
    /// The size in pixels of what is drawn into.
    pub viewport: [f32; 2],
    /// How long a step along an edge should be on screen, in pixels.
    pub pixels: f32,
    /// The word the first patch starts at in the page.
    pub offset: u32,
    /// Patches never get finer than `2^max_level` steps along a side.
    /// At most [`MAX_LOD_LEVEL`].
    pub max_level: u32,
    /// How surfaces are coloured, see [`LodColors`].
    pub debug: u32,
    /// How many patches there are.
    pub patches: u32,
    pub _padding: u32,
}
