use std::sync::Arc;

const HOST: &str = r#"
struct Signals {
    size: vec2<f32>, time: f32, audio: f32,
    pointer: vec2<f32>, parallax: f32, detail: f32,
    tint: vec4<f32>, image_size: vec2<f32>, brightness: f32, pad: f32,
    view: vec4<f32>
}
@group(0) @binding(0) var<uniform> signals: Signals;
@group(0) @binding(1) var wallpaper_image: texture_2d<f32>;
@group(0) @binding(2) var wallpaper_sampler: sampler;
@group(0) @binding(3) var wallpaper_depth: texture_2d<f32>;
override serein_encode_srgb: bool = false;

fn serein_linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055,
                  linear * 12.92, linear <= vec3<f32>(0.0031308));
}

fn image_parallax(uv: vec2<f32>, pointer: vec2<f32>) -> vec3<f32> {
    let screen_aspect = signals.size.x / max(signals.size.y, 1.0);
    let image_aspect = signals.image_size.x / max(signals.image_size.y, 1.0);
    let crop = vec2<f32>(min(screen_aspect / image_aspect, 1.0), min(image_aspect / screen_aspect, 1.0));
    let base = (uv - 0.5) * crop * (1.0 - signals.parallax * 2.0) + 0.5;
    var offset = base;
    // A small fixed iteration count keeps depth warping predictable.
    for (var i = 0; i < 4; i++) {
        let depth = textureSampleLevel(wallpaper_depth, wallpaper_sampler, clamp(offset, vec2<f32>(0.002), vec2<f32>(0.998)), 0.0).r;
        offset = base + (pointer - 0.5) * 0.035 * (depth - 0.35) * 2.0;
    }
    return textureSampleLevel(wallpaper_image, wallpaper_sampler, clamp(offset, vec2<f32>(0.002), vec2<f32>(0.998)), 0.0).rgb;
}
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Vertex {
    let x = f32((index << 1u) & 2u); let y = f32(index & 2u);
    var result: Vertex;
    result.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    result.uv = vec2<f32>(x, y); return result;
}
@fragment fn fs_main(input: Vertex) -> @location(0) vec4<f32> {
    let uv = signals.view.xy + input.uv * signals.view.zw;
    let scaled_uv = (uv - 0.5) / signals.detail + 0.5;
    let controlled_pointer = 0.5 + (signals.pointer - 0.5) * (signals.parallax / 0.035);
    let color = shade(scaled_uv, signals.time, signals.audio, controlled_pointer);
    var output = clamp(color * signals.tint.rgb * signals.brightness, vec3<f32>(0.0), vec3<f32>(1.0));
    // sRGB targets encode in hardware. Iced's default web-colors target is
    // plain UNORM, so it needs exactly one explicit encoding at this boundary.
    if serein_encode_srgb { output = serein_linear_to_srgb(output); }
    return vec4<f32>(output, 1.0);
}
"#;

pub fn compile(source: &str) -> Result<Arc<str>, String> {
    if source.len() > 256 * 1024 {
        return Err("Shader exceeds 256 KiB. Reduce its size and compile again.".into());
    }
    let combined = format!("{source}\n{HOST}");
    let module =
        naga::front::wgsl::parse_str(&combined).map_err(|e| e.emit_to_string(&combined))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| e.emit_to_string(&combined))?;
    if module.entry_points.len() != 2
        || module
            .global_variables
            .iter()
            .filter(|(_, v)| v.binding.is_some())
            .count()
            != 4
    {
        return Err("Use fn shade(uv, time, audio, pointer) → vec3<f32>. The host owns resource bindings and entry points.".into());
    }
    Ok(combined.into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn published_creator_examples_compile() {
        let guide = include_str!("../../../docs/07-creator-guide.md");
        for example in guide.split("```wgsl\n").skip(1) {
            let source = example.split("```").next().expect("code block");
            super::compile(source).unwrap_or_else(|error| panic!("Creator guide: {error}"));
        }
    }
    #[test]
    fn catalog_matches_the_runtime_interface() {
        for preset in super::super::catalog::PRESETS {
            super::compile(preset.shader)
                .unwrap_or_else(|error| panic!("{}: {error}", preset.name));
        }
    }
    #[test]
    fn rejects_invalid_code_and_extra_bindings() {
        assert!(super::compile("fn shade() { wrong }").is_err());
        assert!(
            super::compile(&format!(
                "{}\n@group(1) @binding(0) var extra: texture_2d<f32>;",
                serein_domain::DEFAULT_SHADER
            ))
            .is_err()
        );
    }
}
