use serein_domain::{Appearance, ImageLayer};
use serein_render::{Material, Uniforms, compile, headless_device};

// This reads back actual GPU output, so a transfer-function regression cannot
// pass merely because the same incorrect formula appears in two CPU helpers.
#[test]
#[ignore = "requires an available graphics adapter; run explicitly for rendering changes"]
fn preview_and_desktop_encode_the_same_pixels() {
    let (device, queue) = pollster::block_on(headless_device()).expect("GPU");
    let source = compile("fn shade(uv: vec2<f32>, time: f32, audio: f32, pointer: vec2<f32>) -> vec3<f32> { return vec3<f32>(0.0031308, 0.018, 0.5); }").unwrap();
    let read = |format, source: &str, photo: Option<&ImageLayer>| {
        let material = Material::new(&device, &queue, format, source, photo).unwrap();
        material.update(
            &queue,
            Uniforms::new([1.; 2], 0., 0., [0.5; 2], &Appearance::default()),
        );
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Color parity target"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Color parity readback"),
            size: 256,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            material.draw(&mut pass);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap();
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receive.recv().unwrap().unwrap();
        let pixel: [u8; 4] = buffer.slice(..).get_mapped_range()[..4].try_into().unwrap();
        buffer.unmap();
        pixel
    };
    let assert_pixel = |actual: [u8; 4], expected: [u8; 4]| {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "channel: {actual}, expected {expected}"
            );
        }
    };
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        assert_pixel(read(format, &source, None), [10, 36, 188, 255]);
    }
    for preset in serein_render::catalog::PRESETS {
        let source = compile(preset.shader).unwrap();
        let preview = read(wgpu::TextureFormat::Rgba8Unorm, &source, None);
        let desktop = read(wgpu::TextureFormat::Rgba8UnormSrgb, &source, None);
        assert_pixel(preview, desktop);
    }
    let path = std::env::temp_dir().join(format!("serein-color-test-{}.png", std::process::id()));
    image::RgbaImage::from_pixel(1, 1, image::Rgba([64, 128, 192, 255]))
        .save(&path)
        .unwrap();
    let photo = ImageLayer {
        path: path.clone(),
        depth_path: None,
    };
    let source = compile(serein_render::catalog::IMAGE_SHADER).unwrap();
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ] {
        assert_pixel(read(format, &source, Some(&photo)), [64, 128, 192, 255]);
    }
    std::fs::remove_file(path).unwrap();
}
