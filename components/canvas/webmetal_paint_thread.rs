/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasCommonMsg, CanvasData, CanvasImageData, CanvasMsg, byte_swap};
use canvas_traits::{FromLayoutMsg, FromScriptMsg};
use canvas_traits::{WebMetalCommand, WebMetalDeviceRequest, WebMetalEncoderCommand, WebMetalInit};
use euclid::size::Size2D;
use ipc_channel::ipc::{self, IpcSender};
use std::collections::VecDeque;
use std::collections::hash_map::{Entry, HashMap};
use std::slice;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use util::thread::spawn_named;
use webmetal::{self, WebMetalCapabilities};
use webrender_traits;

fn _time<U, F: FnOnce() -> U>(what: &str, fun: F) -> U {
    use std::time;
    let st = time::SystemTime::now();
    let u = fun();
    println!("Time for {}: {:?}", what, st.elapsed().unwrap());
    u
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct UniqueFenceKey(u64);

/// This tracker keeps an eye on all the queued command buffers,
/// associates them with the internally-managed fences, and
/// ensures that the recycled command buffers are no longer used.
struct CommandBufferTracker {
    queue: webmetal::Queue,
    pending: VecDeque<(webmetal::CommandBuffer, webmetal::Fence, UniqueFenceKey)>,
    fences: Vec<webmetal::Fence>,
    unique_key: UniqueFenceKey,
}

impl CommandBufferTracker {
    fn new(queue: webmetal::Queue) -> CommandBufferTracker {
        CommandBufferTracker {
            queue: queue,
            pending: VecDeque::new(),
            fences: Vec::new(),
            unique_key: UniqueFenceKey(0),
        }
    }

    fn find(&self, unique_key: UniqueFenceKey) -> Option<&webmetal::Fence> {
        self.pending.iter().find(|&&(_, _, key)| key == unique_key)
                           .map(|&(_, ref fence, _)| fence)
    }

    fn consume(&mut self,
               com: webmetal::CommandBuffer,
               device: &webmetal::Device) -> UniqueFenceKey {
        let fence = match self.fences.pop() {
            Some(fence) => fence,
            None => device.make_fence(false),
        };
        self.queue.submit(&device.share, &com, Some(&fence));
        self.unique_key.0 += 1;
        self.pending.push_back((com, fence, self.unique_key));
        self.unique_key
    }

    fn produce(&mut self, device: &webmetal::Device)
               -> webmetal::CommandBuffer {
        let is_ready = match self.pending.front() {
            Some(&(_, ref fence, _)) => device.check_fence(fence),
            _ => false,
        };
        let com = if is_ready {
            let (com, fence, _) = self.pending.pop_front().unwrap();
            self.fences.push(fence);
            com
        } else {
            device.make_command_buffer(&self.queue)
        };
        com.begin(&device.share);
        com
    }
}

/// This tracker keeps an eye on the encoder threads for the active
/// command buffers, allowing to wait for them to be done before
/// proceeding with the command buffers on the paint thread.
struct RenderEncoderTracker {
    active_encoders: HashMap<webmetal::CommandBuffer, Receiver<()>>,
}

impl RenderEncoderTracker {
    fn new() -> RenderEncoderTracker {
        RenderEncoderTracker {
            active_encoders: HashMap::new(),
        }
    }

    fn add(&mut self, com: &webmetal::CommandBuffer) -> Sender<()> {
        let (sender, receiver) = channel();
        match self.active_encoders.entry(com.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(receiver);
            },
            Entry::Occupied(mut entry) => {
                let _ = entry.get().recv().unwrap(); //wait
                *entry.get_mut() = receiver;
            },
        }
        sender
    }

    fn sub(&mut self, com: &webmetal::CommandBuffer) {
        if let Entry::Occupied(entry) = self.active_encoders.entry(com.clone()) {
            let _ = entry.remove().recv().unwrap();
        }
    }
}

struct RenderEncoderThread {
    share: Arc<webmetal::Share>,
    com: webmetal::CommandBuffer,
    res: webmetal::ResourceState,
    _pass: webmetal::RenderPass,
    _framebuf: webmetal::FrameBuffer,
}

impl RenderEncoderThread {
    fn new(share: &Arc<webmetal::Share>, com: webmetal::CommandBuffer,
           pass: webmetal::RenderPass, framebuf: webmetal::FrameBuffer,
           clear_data: webmetal::FrameClearData)
           -> RenderEncoderThread {
        com.begin_pass(share, &pass, &framebuf, clear_data);
        RenderEncoderThread {
            share: share.clone(),
            com: com,
            res: webmetal::ResourceState::new(),
            _pass: pass,
            _framebuf: framebuf,
        }
    }

    fn handle_message(&mut self, message: WebMetalEncoderCommand) -> bool {
        match message {
            WebMetalEncoderCommand::SetPipeline(pipeline) => {
                self.com.bind_pipeline(&self.share, &pipeline);
            }
            WebMetalEncoderCommand::Draw(start, count, instances) => {
                self.com.draw(&self.share, start, count, instances);
            },
            WebMetalEncoderCommand::EndEncoding => {
                self.com.end_pass(&self.share);
                self.com.reset_state(&self.share, &mut self.res);
                return false;
            }
        }
        true
    }
}

pub struct WebMetalPaintThread {
    device: webmetal::Device,
    swap_chain: webmetal::SwapChain,
    swap_fence_key: UniqueFenceKey,
    cbuf_tracker: CommandBufferTracker,
    encoder_tracker: RenderEncoderTracker,
    _size: Size2D<i32>,
    wr_api: webrender_traits::RenderApi,
    final_image: webrender_traits::ImageKey,
}

impl WebMetalPaintThread {
    fn new(size: Size2D<i32>, frame_num: u8,
           wr_api_sender: webrender_traits::RenderApiSender)
           -> Result<(WebMetalPaintThread, WebMetalCapabilities), String> {
        let debug = false; //use command line instead for now
        match webmetal::Device::new(debug) {
            Ok((dev, queue, caps)) => {
                let gpu_frame_count = 1; // no need for more when doing a readback
                let swap_chain = dev.make_swap_chain(size.width as u32,
                                                     size.height as u32,
                                                     gpu_frame_count,
                                                     frame_num as u32);
                let wr_api = wr_api_sender.create_api();
                let image_key = wr_api.alloc_image();
                let painter = WebMetalPaintThread {
                    device: dev,
                    swap_chain: swap_chain,
                    swap_fence_key: UniqueFenceKey(0),
                    cbuf_tracker: CommandBufferTracker::new(queue),
                    encoder_tracker: RenderEncoderTracker::new(),
                    _size: size,
                    wr_api: wr_api,
                    final_image: image_key,
                };
                Ok((painter, caps))
            }
            Err(e) => Err(format!("{:?}", e))
        }
    }

    fn init(&mut self) {
        //WM TODO
    }

    fn handle_device_request(&mut self, request: WebMetalDeviceRequest) {
        match request {
            WebMetalDeviceRequest::MakeCommandBuffer(sender) => {
                let com = self.cbuf_tracker.produce(&self.device);
                sender.send(Some(com)).unwrap();
            }
            WebMetalDeviceRequest::MakeRenderPass(sender, targets) => {
                let (pass, clear_data) = self.device.make_render_pass(&targets);
                let framebuf = self.device.make_frame_buffer(&targets, &pass);
                sender.send(Some((pass, framebuf, clear_data))).unwrap();
            }
            WebMetalDeviceRequest::MakeShader(sender, code, stype) => {
                let shader = self.device.make_shader(&code, stype);
                sender.send(Some(shader)).unwrap();
            }
            WebMetalDeviceRequest::MakeRenderPipeline(sender, desc, pass) => {
                let pso_layout = self.device.make_pipeline_layout();
                let pso = self.device.make_pipeline(&desc, &pso_layout, &pass);
                sender.send(Some(pso)).unwrap();
            }
        }
    }

    fn handle_message(&mut self, message: WebMetalCommand) {
        debug!("WebMetal message: {:?}", message);
        match message {
            WebMetalCommand::Device(request) => {
                self.handle_device_request(request);
            }
            WebMetalCommand::StartRenderEncoder(receiver, com, pass, framebuf, clear_data) => {
                let done_sender = self.encoder_tracker.add(&com);
                let mut thread = RenderEncoderThread::new(&self.device.share, com, pass, framebuf, clear_data);
                spawn_named("RenderEncoder".to_owned(), move || {
                    while let Ok(message) = receiver.recv() {
                        if !thread.handle_message(message) {
                            done_sender.send(()).unwrap();
                            return;
                        }
                    }
                });
            }
            WebMetalCommand::Present(frame_index) => {
                let mut res = webmetal::ResourceState::new();
                let com = self.cbuf_tracker.produce(&self.device);
                self.swap_chain.fetch_frame(&self.device.share, &mut res, &com, frame_index);
                com.reset_state(&self.device.share, &mut res);
                self.swap_fence_key = self.cbuf_tracker.consume(com, &self.device);
            }
            WebMetalCommand::Submit(com) => {
                self.encoder_tracker.sub(&com);
                self.cbuf_tracker.consume(com, &self.device);
            }
        }
    }

    /// Creates a new `WebMetalPaintThread` and returns an `IpcSender` to
    /// communicate with it.
    pub fn start(size: Size2D<i32>, frame_num: u8,
                 wr_api_sender: webrender_traits::RenderApiSender)
                 -> Result<WebMetalInit, String> {
        let (sender, receiver) = ipc::channel::<CanvasMsg>().unwrap();
        let (result_chan, result_port) = channel();
        spawn_named("WebMetalThread".to_owned(), move || {
            let mut painter = match WebMetalPaintThread::new(size, frame_num, wr_api_sender) {
                Ok((thread, caps)) => {
                    let targets = thread.swap_chain.get_targets();
                    result_chan.send(Ok((caps, targets))).unwrap();
                    thread
                },
                Err(e) => {
                    result_chan.send(Err(e)).unwrap();
                    return
                }
            };
            painter.init();

            while let Ok(canvas_msg) = receiver.recv() {
                match canvas_msg {
                    CanvasMsg::WebMetal(message) => {
                        painter.handle_message(message);
                    }
                    CanvasMsg::Common(message) => {
                        match message {
                            CanvasCommonMsg::Close => break,
                            CanvasCommonMsg::Recreate(size) => painter.recreate(size).unwrap(),
                        }
                    }
                    CanvasMsg::FromScript(message) => {
                        match message {
                            FromScriptMsg::SendPixels(chan) => {
                                chan.send(None).unwrap();
                            }
                        }
                    }
                    CanvasMsg::FromLayout(message) => {
                        match message {
                            FromLayoutMsg::SendData(chan) => {
                                painter.send_data(chan);
                            }
                        }
                    }
                    CanvasMsg::Canvas2d(_) |
                    CanvasMsg::WebGL(_) => panic!("Wrong message sent to WebMetalThread"),
                }
            }
        });

        result_port.recv().unwrap().map(|(caps, targets)| (sender, targets, caps))
    }

    #[allow(unsafe_code)]
    fn send_data(&mut self, chan: IpcSender<CanvasData>) {
        // wait for the associated command buffer to finish execution
        if let Some(fence) = self.cbuf_tracker.find(self.swap_fence_key) {
            let success = self.device.wait_fence(fence, 100000000);
            assert!(success);
        }

        // read the frame into a vector
        let dim = self.swap_chain.get_dimensions();
        let frame = self.device.read_frame(&self.swap_chain);

        let orig_pixels = unsafe {
            slice::from_raw_parts(frame.pointer, frame.size as usize)
        };

        // flip image vertically (texture is upside down)
        let mut pixels = orig_pixels.to_owned();
        let stride = dim.w as usize * 4;
        for y in 0 .. dim.h as usize {
            let dst_start = y * stride;
            let src_start = (dim.h as usize - y - 1) * stride;
            let src_slice = &orig_pixels[src_start .. src_start + stride];
            (&mut pixels[dst_start .. dst_start + stride]).clone_from_slice(&src_slice[..stride]);
        }

        // rgba -> bgra
        byte_swap(&mut pixels);

        self.wr_api.update_image(self.final_image, dim.w, dim.h,
                                 webrender_traits::ImageFormat::RGBA8,
                                 pixels);

        let image_data = CanvasImageData {
            image_key: self.final_image,
        };

        chan.send(CanvasData::Image(image_data)).unwrap();
    }

    fn recreate(&mut self, _size: Size2D<i32>) -> Result<(), &'static str> {
        //WM TODO
        Ok(())
    }
}

impl Drop for WebMetalPaintThread {
    fn drop(&mut self) {
        //WM TODO
    }
}
