use serein_domain::{Appearance, ImageLayer};
use std::{path::Path, sync::Arc};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub size: [f32; 2],
    pub time: f32,
    pub audio: f32,
    pub pointer: [f32; 2],
    pub parallax: f32,
    pub detail: f32,
    pub tint: [f32; 4],
    pub image_size: [f32; 2],
    pub brightness: f32,
    pub pad: f32,
    pub view: [f32; 4],
}
impl Uniforms {
    pub fn new(
        size: [f32; 2],
        time: f32,
        audio: f32,
        pointer: [f32; 2],
        look: &Appearance,
    ) -> Self {
        let tint = match look.palette {
            1 => [1.3, 0.8, 0.65, 1.],
            2 => [0.65, 1.05, 1.3, 1.],
            3 => [1.05, 0.7, 1.3, 1.],
            _ => [1.; 4],
        };
        Self {
            size,
            time: time * look.speed,
            audio: (audio * look.audio_amount).clamp(0., 5.),
            pointer,
            parallax: look.parallax,
            detail: look.detail,
            tint,
            image_size: [1.; 2],
            brightness: look.brightness,
            pad: 0.,
            view: [0., 0., 1., 1.],
        }
    }
}

pub struct Material {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub uniforms: wgpu::Buffer,
    pub image_size: [f32; 2],
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        source: &str,
        image: Option<&ImageLayer>,
    ) -> Result<Self, String> {
        let photo = match image {
            Some(layer) => load_image(&layer.path)?,
            None => image::RgbaImage::from_pixel(1, 1, image::Rgba([255; 4])),
        };
        let depth = match image.and_then(|layer| layer.depth_path.as_ref()) {
            Some(path) => load_image(path)?,
            None => approximate_depth(&photo),
        };
        let image_size = [photo.width() as f32, photo.height() as f32];
        let photo_view = upload(
            device,
            queue,
            &photo,
            "Wallpaper image",
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let depth_view = upload(
            device,
            queue,
            &depth,
            "Wallpaper depth",
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Wallpaper sampling"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Wallpaper signals"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Wallpaper ABI"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Uniforms>() as u64
                        ),
                    },
                    count: None,
                },
                texture_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                texture_entry(3),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wallpaper inputs"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&photo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Wallpaper pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wallpaper shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Wallpaper"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[(
                        "serein_encode_srgb",
                        if format.is_srgb() { 0.0 } else { 1.0 },
                    )],
                    ..Default::default()
                },
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Ok(Self {
            pipeline,
            bind_group,
            uniforms,
            image_size,
        })
    }
    pub fn update(&self, queue: &wgpu::Queue, mut signals: Uniforms) {
        signals.image_size = self.image_size;
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&signals));
    }
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

pub fn load_image(path: &Path) -> Result<image::RgbaImage, String> {
    let reader = image::ImageReader::open(path)
        .map_err(|e| format!("Unable to open {}: {e}", path.display()))?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut reader = reader;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader
        .decode()
        .map_err(|e| {
            format!(
                "Unable to decode image: {e}. Use a PNG, JPEG or WebP up to 8192 pixels per side."
            )
        })?
        .to_rgba8())
}

pub fn save_image_thumbnail(photo: &image::RgbaImage, path: &Path) -> Result<(), String> {
    image::imageops::thumbnail(photo, 384, 216)
        .save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
}

fn approximate_depth(photo: &image::RgbaImage) -> image::RgbaImage {
    // This is a local compositional heuristic, not semantic/AI depth estimation.
    let small = image::imageops::thumbnail(photo, 256, 256);
    let smooth = image::imageops::blur(&small, 8.);
    image::RgbaImage::from_fn(smooth.width(), smooth.height(), |x, y| {
        let p = smooth.get_pixel(x, y);
        let u = x as f32 / smooth.width() as f32;
        let v = y as f32 / smooth.height() as f32;
        let center = (1. - ((u - 0.5).hypot((v - 0.5) * 0.7) * 1.8)).clamp(0., 1.);
        let luma = (p[0] as f32 * 0.2126 + p[1] as f32 * 0.7152 + p[2] as f32 * 0.0722) / 255.;
        let depth = ((center * 0.55 + v * 0.25 + luma * 0.2) * 255.) as u8;
        image::Rgba([depth, depth, depth, 255])
    })
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &image::RgbaImage,
    label: &str,
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        image.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(image.width() * 4),
            rows_per_image: Some(image.height()),
        },
        texture.size(),
    );
    texture.create_view(&Default::default())
}

pub async fn headless_device() -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>), String> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|e| e.to_string())?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|e| e.to_string())?;
    Ok((Arc::new(device), Arc::new(queue)))
}
