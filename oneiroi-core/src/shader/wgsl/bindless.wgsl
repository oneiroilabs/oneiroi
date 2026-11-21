struct Page {
    words: array<u32>,
}

@group(0) @binding(0) var<storage, read_write> pages: binding_array<Page>;
@group(1) @binding(0) var<storage, read> calls: array<u32>;
