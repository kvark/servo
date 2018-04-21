use std::slice;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};

use webrender_api as wrapi;
use webrender::{ExternalImage, ExternalImageHandler, ExternalImageSource};

use super::Swapchain;
use canvas_traits::hal;


struct Framebuffer<B: hal::Backend> {
    gpu: Arc<hal::Gpu<B>>,
    swapchain: Arc<Mutex<Swapchain<B>>>,
}

pub enum Message<B: hal::Backend> {
    Add(Arc<hal::Gpu<B>>, Arc<Mutex<Swapchain<B>>>),
    Remove,
}

pub struct FrameHandler<B: hal::Backend> {
    channel: mpsc::Receiver<(wrapi::ExternalImageId, Message<B>)>,
    framebuffers: HashMap<wrapi::ExternalImageId, Framebuffer<B>>,
}

impl<B: hal::Backend> FrameHandler<B> {
    pub fn new() -> (Box<ExternalImageHandler>, mpsc::Sender<(wrapi::ExternalImageId, Message<B>)>) {
        let (sender, channel) = mpsc::channel();
        let handler = FrameHandler {
            channel,
            framebuffers: HashMap::new(),
        };
        (Box::new(handler), sender)
    }

    fn update(&mut self) {
        debug!("updating");
        while let Ok((id, message)) = self.channel.try_recv() {
            match message {
                Message::Add(gpu, swapchain) => {
                    self.framebuffers.insert(id, Framebuffer {
                        gpu,
                        swapchain
                    });
                }
                Message::Remove => {
                    self.framebuffers.remove(&id);
                }
            }
        }
    }
}

impl<B: hal::Backend> ExternalImageHandler for FrameHandler<B> {
    #[allow(unsafe_code)]
    fn lock(&mut self, id: wrapi::ExternalImageId, channel_index: u8) -> ExternalImage {
        self.update();
        debug!("entering lock for {:?}", id);
        assert_eq!(channel_index, 0);

        let result = ExternalImage {
            uv: wrapi::TexelRect::new(0.0, 0.0, 1.0, 1.0),
            source: ExternalImageSource::RawData(&[]),
        };

        let fb = match self.framebuffers.get_mut(&id) {
            Some(fb) => fb,
            None => {
                error!("Unknown {:?} handler", id);
                return result;
            }
        };

        let mut sc = fb.swapchain.lock().unwrap();
        match sc.read(&fb.gpu.device) {
            Some(ptr) => {
                let total_size = sc.frame_size() as usize;
                ExternalImage {
                    source: ExternalImageSource::RawData(unsafe {
                        slice::from_raw_parts(ptr, total_size)
                    }),
                    ..result
                }
            }
            None => {
                error!("Unable to read a frame");
                result
            }
        }
    }

    fn unlock(&mut self, id: wrapi::ExternalImageId, channel_index: u8) {
        debug!("entering unlock for {:?}", id);
        assert_eq!(channel_index, 0);

        let fb = match self.framebuffers.get_mut(&id) {
            Some(fb) => fb,
            None => {
                error!("Unknown {:?} handler", id);
                return
            }
        };

        let mut sc = fb.swapchain.lock().unwrap();
        sc.read_done(&fb.gpu.device);
    }
}
