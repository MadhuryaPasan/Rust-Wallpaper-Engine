use serein_domain::Appearance;
use serein_render::{Material, Uniforms, catalog::PRESETS, compile, headless_device};
fn main() -> Result<(), String> {
    let output = std::env::args()
        .nth(1)
        .ok_or("Pass the thumbnail output directory")?;
    std::fs::create_dir_all(&output).map_err(|e| e.to_string())?;
    let (device, queue) = pollster::block_on(headless_device())?;
    let width = 384;
    let height = 216;
    let stride = width * 4;
    for preset in PRESETS {
        let source = compile(preset.shader)?;
        let material = Material::new(
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &source,
            None,
        )?;
        material.update(
            &queue,
            Uniforms::new(
                [width as f32, height as f32],
                7.,
                0.25,
                [0.5, 0.5],
                &Appearance::default(),
            ),
        );
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(preset.name),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Thumbnail readback"),
            size: (stride * height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Catalog preview"),
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
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        rx.recv()
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let bytes = buffer.slice(..).get_mapped_range().to_vec();
        buffer.unmap();
        image::save_buffer(
            format!("{output}/{}.png", preset.id),
            &bytes,
            width,
            height,
            image::ColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
        println!("Rendered {}", preset.name);
    }
    Ok(())
}
