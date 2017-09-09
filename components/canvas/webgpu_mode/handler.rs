use std::slice;
use std::collections::HashMap;
use std::sync::Arc;

use webrender_api as wrapi;
use webrender::{ExternalImage, ExternalImageHandler, ExternalImageSource};

use super::resource::ResourceHub;
use canvas_traits::webgpu as w;
use webgpu::gpu::{self, Device};


struct FrameQueue<B: gpu::Backend> {
    next: Option<w::ReadyFrame>,
    locked: Option<(w::ReadyFrame, B::Mapping)>,
}

pub struct FrameHandler<B: gpu::Backend> {
    receiver: w::WebGpuReceiver<(wrapi::ExternalImageId, w::WebGpuPresent)>,
    queues: HashMap<wrapi::ExternalImageId, FrameQueue<B>>, //TODO: faster collection?
    rehub: Arc<ResourceHub<B>>,
}

impl<B: gpu::Backend> FrameHandler<B> {
    pub fn new(rehub: Arc<ResourceHub<B>>) -> (Self, w::WebGpuPresentChan) {
        let (sender, receiver) = w::webgpu_channel().unwrap();
        let handler = FrameHandler {
            receiver,
            queues: HashMap::new(),
            rehub,
        };
        (handler, sender)
    }

    fn update(&mut self) {
        while let Ok((id, present)) = self.receiver.try_recv() {
            match present {
                w::WebGpuPresent::Enter => {
                    self.queues.insert(id, FrameQueue {
                        next: None,
                        locked: None,
                    });
                }
                w::WebGpuPresent::Exit => {
                    self.queues.remove(&id);
                }
                w::WebGpuPresent::Show(frame) => {
                    match self.queues.get_mut(&id) {
                        Some(queue) => {
                            //println!("handler: received frame with buffer id {:?}, replacing old: {}",
                            //    frame.buffer_id, queue.next.is_some());
                            if let Some(old_frame) = queue.next.take() {
                                old_frame.consume(false);
                            }
                            queue.next = Some(frame);
                        }
                        None => {
                            warn!("There is no frame to show for {:?}", id);
                        }
                    }
                }
            }
        }
    }
}

impl<B: gpu::Backend> ExternalImageHandler for FrameHandler<B> {
    #[allow(unsafe_code)]
    fn lock(&mut self, id: wrapi::ExternalImageId, channel_index: u8) -> ExternalImage {
        //println!("entering lock for {:?}", id);
        assert_eq!(channel_index, 0);
        self.update();

        let result = ExternalImage { //TODO
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
            source: ExternalImageSource::RawData(&[]),
        };

        let queue = match self.queues.get_mut(&id) {
            Some(queue) => queue,
            None => {
                error!("Unknown {:?} handler", id);
                return result;
            }
        };

        let frame = match queue.next.take() {
            Some(frame) => frame,
            None => {
                warn!("There is no frame to lock for {:?}", id);
                return result;
            }
        };

        //println!("handler: locking frame with buffer id {:?}", frame.buffer_id);

        let total_size = frame.bytes_per_row * frame.size.height as usize;
        let (ptr, mapping) = {
            let device = &mut self.rehub.gpus.lock().unwrap()[frame.gpu_id].device;
            let fence = &self.rehub.fences.read().unwrap()[frame.fence_id];
            device.wait_for_fences(&[fence], gpu::device::WaitFor::Any, !0); //TEMP
            device.reset_fences(&[fence]);
            let buffer = &self.rehub.buffers.read().unwrap()[frame.buffer_id];
            device.read_mapping_raw(buffer, 0 .. total_size as _).unwrap()
        };

        debug_assert!(queue.locked.is_none());
        queue.locked = Some((frame, mapping));

        ExternalImage {
            source: ExternalImageSource::RawData(unsafe {
                slice::from_raw_parts(ptr, total_size)
            }),
            ..result
        }
    }

    fn unlock(&mut self, id: wrapi::ExternalImageId, channel_index: u8) {
        //println!("entering unlock for {:?}", id);
        assert_eq!(channel_index, 0);
        self.update();

        let queue = match self.queues.get_mut(&id) {
            Some(queue) => queue,
            None => {
                error!("Unknown {:?} handler", id);
                return
            }
        };

        let (frame, mapping) = match queue.locked.take() {
            Some(frame) => frame,
            None => {
                warn!("There is no frame to unlock for {:?}", id);
                return;
            }
        };

        //println!("handler: unlocking frame with buffer id {:?}", frame.buffer_id);

        //TODO: let the parent know a frame is free to be reused?
        let device = &mut self.rehub.gpus.lock().unwrap()[frame.gpu_id].device;
        device.unmap_mapping_raw(mapping);

        if queue.next.is_none() {
            queue.next = Some(frame.reuse());
        } else {
            frame.consume(true);
        }
    }
}
