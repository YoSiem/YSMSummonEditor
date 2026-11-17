//! Icons and illustrations from the dump folder.
//!
//! Images are resolved, decoded and scaled on a background thread and
//! uploaded as textures when ready. Until then (and when a file is missing)
//! icons show the built-in placeholder.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, ColorImage, TextureHandle, TextureId, TextureOptions};
use image::imageops::FilterType;
use ysm_data::AssetIndex;

/// Icons are shown at 36 px; keep enough pixels for high-DPI screens.
const ICON_PIXELS: u32 = 72;
const ILLUSTRATION_MAX_PIXELS: u32 = 720;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Icon,
    Illustration,
}

enum Slot {
    Loading,
    Ready(TextureHandle),
    Missing,
}

enum Request {
    SetIndex { index: Arc<AssetIndex>, generation: u64 },
    Load { kind: Kind, name: String, generation: u64 },
}

struct Loaded {
    kind: Kind,
    name: String,
    generation: u64,
    image: Option<ColorImage>,
}

pub struct TextureStore {
    ctx: egui::Context,
    slots: HashMap<(Kind, String), Slot>,
    requests: Sender<Request>,
    loaded: Receiver<Loaded>,
    /// Bumped whenever the asset index changes, to drop answers for the old one.
    generation: u64,
    fallback_icon: TextureHandle,
    logo: Option<TextureHandle>,
}

impl TextureStore {
    pub fn new(ctx: &egui::Context) -> Self {
        let (request_tx, request_rx) = channel::<Request>();
        let (loaded_tx, loaded_rx) = channel::<Loaded>();
        let repaint = ctx.clone();

        std::thread::Builder::new()
            .name("ysm-images".into())
            .spawn(move || loader_thread(request_rx, loaded_tx, repaint))
            .expect("the image loader thread can be started");

        let embedded = |name: &str, bytes: &[u8], max: u32| {
            let image = image::load_from_memory(bytes).ok()?;
            Some(ctx.load_texture(name, to_color_image(image, max), TextureOptions::LINEAR))
        };
        let fallback_icon = embedded(
            "fallback-icon",
            include_bytes!("../../../assets/common_mark_icon_unknown_middle.jpg"),
            ICON_PIXELS,
        )
        .unwrap_or_else(|| {
            ctx.load_texture(
                "fallback-icon",
                ColorImage::filled([2, 2], egui::Color32::DARK_GRAY),
                TextureOptions::LINEAR,
            )
        });
        let logo = embedded("logo", include_bytes!("../../../assets/Heaven_logo1.png"), 256);

        Self {
            ctx: ctx.clone(),
            slots: HashMap::new(),
            requests: request_tx,
            loaded: loaded_rx,
            generation: 0,
            fallback_icon,
            logo,
        }
    }

    /// Switches to a new asset index, forgetting every image loaded so far.
    pub fn set_index(&mut self, index: Arc<AssetIndex>) {
        self.generation += 1;
        self.slots.clear();
        let _ = self.requests.send(Request::SetIndex { index, generation: self.generation });
    }

    /// Uploads images decoded since the last frame.
    pub fn poll(&mut self) {
        while let Ok(loaded) = self.loaded.try_recv() {
            if loaded.generation != self.generation {
                continue;
            }
            let slot = match loaded.image {
                Some(image) => {
                    let label = format!("{:?}:{}", loaded.kind, loaded.name);
                    Slot::Ready(self.ctx.load_texture(label, image, TextureOptions::LINEAR))
                }
                None => Slot::Missing,
            };
            self.slots.insert((loaded.kind, loaded.name), slot);
        }
    }

    /// A 36 px icon by file name; the placeholder when it is missing or still loading.
    pub fn icon(&mut self, file_name: Option<&str>) -> TextureId {
        let fallback = self.fallback_icon.id();
        self.texture(Kind::Icon, file_name).map_or(fallback, |texture| texture.id())
    }

    /// A summon illustration, once loaded.
    pub fn illustration(&mut self, file_name: Option<&str>) -> Option<&TextureHandle> {
        self.texture(Kind::Illustration, file_name)
    }

    pub fn logo(&self) -> Option<&TextureHandle> {
        self.logo.as_ref()
    }

    fn texture(&mut self, kind: Kind, file_name: Option<&str>) -> Option<&TextureHandle> {
        let name = file_name.map(str::trim).filter(|name| !name.is_empty())?;
        let key = (kind, name.to_lowercase());
        if !self.slots.contains_key(&key) {
            let request = Request::Load { kind, name: key.1.clone(), generation: self.generation };
            let _ = self.requests.send(request);
            self.slots.insert(key.clone(), Slot::Loading);
        }
        match self.slots.get(&key) {
            Some(Slot::Ready(texture)) => Some(texture),
            _ => None,
        }
    }
}

fn loader_thread(requests: Receiver<Request>, loaded: Sender<Loaded>, ctx: egui::Context) {
    let mut index = Arc::new(AssetIndex::empty());
    let mut current_generation = 0;

    for request in requests {
        match request {
            Request::SetIndex { index: new_index, generation } => {
                index = new_index;
                current_generation = generation;
            }
            Request::Load { kind, name, generation } => {
                // Requests queued for a previous index are answered as missing without decoding.
                let image = (generation == current_generation).then(|| decode(&index, kind, &name)).flatten();
                if loaded.send(Loaded { kind, name, generation, image }).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        }
    }
}

fn decode(index: &AssetIndex, kind: Kind, name: &str) -> Option<ColorImage> {
    let path = index.resolve(name)?;
    let image = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?;
    let max = match kind {
        Kind::Icon => ICON_PIXELS,
        Kind::Illustration => ILLUSTRATION_MAX_PIXELS,
    };
    Some(to_color_image(image, max))
}

/// Scales an image down to fit `max` pixels on its longer side and converts it for egui.
fn to_color_image(image: image::DynamicImage, max: u32) -> ColorImage {
    let image =
        if image.width() > max || image.height() > max { image.resize(max, max, FilterType::Triangle) } else { image };
    let rgba = image.into_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
}
