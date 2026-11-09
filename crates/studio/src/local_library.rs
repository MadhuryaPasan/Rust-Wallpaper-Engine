use iced::widget::image::Handle;
use serein_domain::{Project, data_directory};

pub struct LocalWallpaper {
    pub id: String,
    pub project: Project,
    pub thumbnail: Handle,
}
pub fn load() -> Vec<LocalWallpaper> {
    let Ok(entries) = std::fs::read_dir(data_directory().join("library")) else {
        return Vec::new();
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.file_name()));
    entries
        .into_iter()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                return None;
            }
            let project = Project::load(&path).ok()?;
            let image = project.image.as_ref()?;
            Some(LocalWallpaper {
                id: path.file_stem()?.to_string_lossy().into_owned(),
                thumbnail: Handle::from_path(image.path.with_extension("thumb.png")),
                project,
            })
        })
        .collect()
}
