use iced::{Rectangle, mouse, wgpu, widget::shader};
use serein_domain::{Appearance, ImageLayer};
pub use serein_render::compile;
use serein_render::{Material, Uniforms};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Preview {
    pub source: Arc<str>,
    pub time: f32,
    pub audio: f32,
    pub pointer_enabled: bool,
    pub appearance: Appearance,
    pub image: Option<ImageLayer>,
}
impl<Message> shader::Program<Message> for Preview {
    type State = ();
    type Primitive = Frame;
    fn draw(&self, _: &(), cursor: mouse::Cursor, bounds: Rectangle) -> Frame {
        let pointer = if self.pointer_enabled {
            cursor
                .position_in(bounds)
                .map(|p| [p.x / bounds.width, p.y / bounds.height])
                .unwrap_or([0.5, 0.5])
        } else {
            [0.5, 0.5]
        };
        Frame {
            source: self.source.clone(),
            image: self.image.clone(),
            uniforms: Uniforms::new(
                [bounds.width, bounds.height],
                self.time,
                self.audio,
                pointer,
                &self.appearance,
            ),
        }
    }
}
#[derive(Debug)]
pub struct Frame {
    source: Arc<str>,
    image: Option<ImageLayer>,
    uniforms: Uniforms,
}
pub struct Pipeline {
    source: Arc<str>,
    image: Option<ImageLayer>,
    material: Option<Material>,
    format: wgpu::TextureFormat,
}
impl shader::Pipeline for Pipeline {
    fn new(_: &wgpu::Device, _: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self {
            source: Arc::from(""),
            image: None,
            material: None,
            format,
        }
    }
}
impl shader::Primitive for Frame {
    type Pipeline = Pipeline;
    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _: &Rectangle,
        _: &shader::Viewport,
    ) {
        if (!Arc::ptr_eq(&self.source, &pipeline.source) || self.image != pipeline.image)
            && let Ok(material) = Material::new(
                device,
                queue,
                pipeline.format,
                &self.source,
                self.image.as_ref(),
            )
        {
            pipeline.material = Some(material);
            pipeline.source = self.source.clone();
            pipeline.image = self.image.clone();
        }
        if let Some(material) = &pipeline.material {
            material.update(queue, self.uniforms);
        }
    }
    fn draw(&self, pipeline: &Pipeline, pass: &mut wgpu::RenderPass<'_>) -> bool {
        if let Some(material) = &pipeline.material {
            material.draw(pass);
        }
        true
    }
}
