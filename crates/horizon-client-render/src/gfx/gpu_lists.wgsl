// Phase 1's solid section draws made on the GPU (gfx/gpu_lists.rs): per section
// listed, the runs of its quads that face the eye, found as
// `mesher::Sides::visible_from` finds them on the CPU, written into the call
// each of its parts draws in (one per pass group and arena page) after taking
// room there: their indices, in the flat draws' index list, as `flatten_seen`
// does for occlusion culling's phase 2. Solid draws write depth and are tested
// against it, so their order within a call does not change the picture.
struct View {
    // The eye relative to the camera, and 1.0 when there is one (`sides_on`);
    // without, every bucket is drawn (`Sides::every`).
    eye: vec3<f32>,
    has_eye: f32,
}
// Per section listed: its row's id; for each of its parts (up to two) the call
// it draws in, with the part (`PART_*`) in the top bits; its number of parts.
// Then its origin relative to the camera.
struct Record {
    words: vec4<u32>,
    origin: vec4<f32>,
}
@group(0) @binding(0) var<storage, read> records: array<Record>;
// Per id, `ROW` words: its first vertex in its page, then `Sides::gpu_row`
// (14 ends, 7 planes' bits, flags).
@group(0) @binding(1) var<storage, read> rows: array<u32>;
@group(0) @binding(2) var<uniform> view: View;
// Per call `DrawIndexedIndirectArgs`: its index count starts at 0, its first
// index is where its room starts.
@group(0) @binding(3) var<storage, read_write> calls: array<atomic<u32>>;
// The flat draws' index list.
@group(0) @binding(4) var<storage, read_write> out: array<u32>;

const ROW: u32 = 24u;
const LANES: u32 = 64u;
const PART_ALL: u32 = 0u;
const PART_OPAQUE: u32 = 1u;
const PART_CUTOUT: u32 = 2u;
const CALL_MASK: u32 = 0x0fffffffu;
// `sides::MARGIN`.
const MARGIN: f32 = 0.0625;

// Index `k` of a solid draw: quad k / 6's vertices 0 1 2 0 2 3.
fn quad(k: u32) -> u32 {
    let c = k % 6u;
    return (k / 6u) * 4u + select(select(c - 2u, 0u, c == 3u), c, c < 3u);
}

fn plane(row: u32, bucket: u32) -> f32 {
    return bitcast<f32>(rows[row + 15u + bucket]);
}

// `Sides::visible_from`'s `seen`, buckets in mesh order: +X, +Z, other, +Y, -Z, -X, -Y.
fn seen(row: u32, bucket: u32, eye: vec3<f32>) -> bool {
    var faces = true;
    if view.has_eye != 0.0 {
        switch bucket {
            case 0u: { faces = eye.x > plane(row, 0u) - MARGIN; }
            case 1u: { faces = eye.z > plane(row, 1u) - MARGIN; }
            case 3u: { faces = eye.y > plane(row, 3u) - MARGIN; }
            case 4u: { faces = eye.z < plane(row, 4u) + MARGIN; }
            case 5u: { faces = eye.x < plane(row, 5u) + MARGIN; }
            case 6u: { faces = eye.y < plane(row, 6u) + MARGIN; }
            default: {}
        }
    }
    return faces;
}

// A part's runs (first quad, quads): at most 7, as a run ends at a part left
// out and there are 14.
struct Runs {
    n: u32,
    quads: u32,
    runs: array<vec2<u32>, 7>,
}

// `Sides::runs` for `part` of the mesh at `row`, seen from `eye` (relative to
// the section): empty parts do not break a run.
fn part_runs(row: u32, part: u32, eye: vec3<f32>) -> Runs {
    var r: Runs;
    let grouped = (rows[row + 22u] & 1u) != 0u;
    let split = (rows[row + 22u] & 2u) != 0u;
    var start = 0u;
    var open = false;
    var first = 0u;
    for (var n = 0u; n < 14u; n++) {
        let end = rows[row + 1u + n];
        if end == start {
            continue;
        }
        var bucket: u32;
        var cutout: bool;
        if grouped {
            bucket = n % 7u;
            cutout = n >= 7u;
        } else {
            bucket = n / 2u;
            cutout = n % 2u == 1u;
        }
        let wanted = part == PART_ALL || !split
            || (part == PART_OPAQUE && !cutout) || (part == PART_CUTOUT && cutout);
        let take = wanted && seen(row, bucket, eye);
        if take && !open {
            open = true;
            first = start;
        } else if !take && open {
            r.runs[r.n] = vec2(first, start - first);
            r.quads += start - first;
            r.n++;
            open = false;
        }
        start = end;
    }
    if open {
        r.runs[r.n] = vec2(first, start - first);
        r.quads += start - first;
        r.n++;
    }
    return r;
}

fn record_index(group: vec3<u32>) -> u32 {
    return group.y * 65535u + group.x;
}

// The eye relative to `record`'s section.
fn eye_of(record: Record) -> vec3<f32> {
    return view.eye - record.origin.xyz;
}

fn part_word(record: Record, p: u32) -> u32 {
    return select(record.words.z, record.words.y, p == 0u);
}

// Lane 0's runs per part, and where each part's indices start.
var<workgroup> found: array<Runs, 2>;
var<workgroup> at: array<u32, 2>;

// One workgroup per record: lane 0 finds the runs and takes room for their
// indices, then every lane writes them.
@compute @workgroup_size(64)
fn flatten_sides(@builtin(workgroup_id) group: vec3<u32>,
    @builtin(local_invocation_index) lane: u32) {
    let r = record_index(group);
    if r >= arrayLength(&records) {
        return;
    }
    let record = records[r];
    let row = record.words.x * ROW;
    let parts = record.words.w;
    if lane == 0u {
        let eye = eye_of(record);
        for (var p = 0u; p < parts; p++) {
            let word = part_word(record, p);
            let runs = part_runs(row, word >> 28u, eye);
            let call = (word & CALL_MASK) * 5u;
            var begin = 0u;
            if runs.quads > 0u {
                begin = atomicLoad(&calls[call + 2u]) + atomicAdd(&calls[call], runs.quads * 6u);
            }
            found[p] = runs;
            at[p] = begin;
        }
    }
    workgroupBarrier();
    let base = rows[row];
    for (var p = 0u; p < parts; p++) {
        var to = at[p];
        for (var k = 0u; k < found[p].n; k++) {
            let run = found[p].runs[k];
            let first = base + run.x * 4u;
            let count = run.y * 6u;
            for (var i = lane; i < count; i += LANES) {
                out[to + i] = first + quad(i);
            }
            to += count;
        }
    }
}
