use std::slice;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use webrender_api as wrapi;
use webrender::{ExternalImage, ExternalImageHandler, ExternalImageSource};

use super::lazyvec::LazyVec;
use super::resource::ResourceHub;
use canvas_traits::webgpu as w;
use webgpu::gpu::{self, Device};


struct FrameQueue<B: gpu::Backend> {
    gpu_id: w::GpuId,
    others: VecDeque<w::ReadyFrame>,
    ready: Option<w::ReadyFrame>,
    locked: Option<(w::ReadyFrame, B::Mapping)>,
}

impl<B: gpu::Backend> FrameQueue<B> {
    fn collapse(&mut self, device: &mut B::Device, fence_store: &LazyVec<B::Fence>) {
        loop {
            let fence = match self.others.front() {
                Some(frame) => &fence_store[frame.fence_id],
                None => return,
            };
            if !device.wait_for_fences(&[fence], gpu::device::WaitFor::Any, 0) {
                return
            }
            device.reset_fences(&[fence]);
            if let Some(ready) = self.ready.take() {
                ready.consume(false);
            }
            self.ready = self.others.pop_front();
        }
    }
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

    fn update(&mut self, fence_store: &LazyVec<B::Fence>) {
        //TODO: communicate if the queue has been processed
        while let Ok((id, present)) = self.receiver.try_recv() {
            match present {
                w::WebGpuPresent::Enter(gpu_id) => {
                    self.queues.insert(id, FrameQueue {
                        gpu_id,
                        others: VecDeque::new(),
                        ready: None,
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
                            let device = &mut self.rehub.gpus.lock().unwrap()[queue.gpu_id].device;
                            queue.others.push_back(frame);
                            queue.collapse(device, fence_store);
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
        let rehub = Arc::clone(&self.rehub);
        let fence_store = rehub.fences.read().unwrap();
        self.update(&*fence_store);

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

        let device = &mut self.rehub.gpus.lock().unwrap()[queue.gpu_id].device;
        queue.collapse(device, &*fence_store);

        let frame = match queue.ready.take() {
            Some(frame) => frame,
            None => match queue.others.pop_front() {
                Some(frame) => {
                    // force wait for a frame
                    let fence = &fence_store[frame.fence_id];
                    device.wait_for_fences(&[fence], gpu::device::WaitFor::Any, !0);
                    device.reset_fences(&[fence]);
                    frame
                }
                None => {
                    warn!("There is no frame to lock for {:?}", id);
                    return result;
                }
            }
        };

        //println!("handler: locking frame with buffer id {:?}", frame.buffer_id);

        let total_size = frame.bytes_per_row * frame.size.height as usize;
        let (ptr, mapping) = {
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
        let rehub = Arc::clone(&self.rehub);
        let fence_store = rehub.fences.read().unwrap();
        self.update(&*fence_store);

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

        let device = &mut self.rehub.gpus.lock().unwrap()[queue.gpu_id].device;
        device.unmap_mapping_raw(mapping);
        queue.collapse(device, &*fence_store);

        if queue.ready.is_none() {
            queue.ready = Some(frame.reuse());
        } else {
            frame.consume(true);
        }
    }
}
