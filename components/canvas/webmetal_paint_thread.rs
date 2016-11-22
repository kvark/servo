/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasCommonMsg, CanvasData, CanvasImageData, CanvasMsg, byte_swap};
use canvas_traits::{FromLayoutMsg, FromScriptMsg};
use canvas_traits::{WebMetalCommand, WebMetalEncoderCommand, WebMetalInit};
use euclid::size::Size2D;
use ipc_channel::ipc::{self, IpcSender};
use std::slice;
use std::sync::mpsc::channel;
use util::thread::spawn_named;
use webmetal::{self, WebMetalCapabilities};
use webrender_traits;

pub struct WebMetalPaintThread {
    device: webmetal::Device,
    queue: webmetal::Queue,
    swap_chain: webmetal::SwapChain,
    service_com: webmetal::CommandBuffer,
    _size: Size2D<i32>,
    wr_api: webrender_traits::RenderApi,
    final_image: webrender_traits::ImageKey,
}

impl WebMetalPaintThread {
    fn new(size: Size2D<i32>, frame_num: u8,
           wr_api_sender: webrender_traits::RenderApiSender)
           -> Result<(WebMetalPaintThread, WebMetalCapabilities), String> {
        match webmetal::Device::new(false) {
            Ok((dev, queue, caps)) => {
                let swap_chain = dev.make_swap_chain(size.width as u32,
                                                     size.height as u32,
                                                     frame_num as u32);
                let com = dev.make_command_buffer(&queue);
                let wr_api = wr_api_sender.create_api();
                let image_key = wr_api.alloc_image();
                let painter = WebMetalPaintThread {
                    device: dev,
                    queue: queue,
                    swap_chain: swap_chain,
                    service_com: com,
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

    fn handle_message(&mut self, message: WebMetalCommand) {
        debug!("WebMetal message: {:?}", message);
        match message {
            WebMetalCommand::MakeCommandBuffer(sender) => {
                let com = self.device.make_command_buffer(&self.queue);
                com.begin(&self.device.share);
                sender.send(Some(com)).unwrap();
            }
            WebMetalCommand::MakeRenderEncoder(receiver, com, targets) => {
                let share = self.device.share.clone();
                spawn_named("RenderEncoder".to_owned(), move || {
                    let com = com;
                    let mut res = webmetal::ResourceState::new();
                    while let Ok(message) = receiver.recv() {
                        match message {
                            WebMetalEncoderCommand::ClearColor(color) => {
                                for view in targets.colors.iter() {
                                    com.clear_color(&share, &mut res, view, color);
                                }
                            }
                            WebMetalEncoderCommand::EndEncoding => {
                                com.reset_state(&share, res);
                                return;
                            }
                        }
                    }
                });
            }
            WebMetalCommand::Present(frame_index) => {
                //TODO: fence
                let mut res = webmetal::ResourceState::new();
                self.service_com.begin(&self.device.share);
                self.swap_chain.fetch_frame(&self.device.share, &mut res, &self.service_com, frame_index);
                self.device.execute(&self.queue, &self.service_com);
            }
            WebMetalCommand::Submit(com) => {
                self.device.execute(&self.queue, &com);
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
