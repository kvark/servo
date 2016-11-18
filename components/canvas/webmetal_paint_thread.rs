/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasCommonMsg, CanvasData, CanvasMsg};
use canvas_traits::{FromLayoutMsg, FromScriptMsg};
use canvas_traits::{WebMetalCommand, WebMetalInit};
use euclid::size::Size2D;
use ipc_channel::ipc::{self, IpcSender};
use std::sync::mpsc::channel;
use util::thread::spawn_named;
use webmetal::{self, WebMetalCapabilities};

pub struct WebMetalPaintThread {
    device: webmetal::Device,
    queue: webmetal::Queue,
    swap_chain: webmetal::SwapChain,
    _size: Size2D<i32>,
}

impl WebMetalPaintThread {
    fn new(size: Size2D<i32>, frame_num: u8)
        -> Result<(WebMetalPaintThread, WebMetalCapabilities), String> {
        match webmetal::Device::new(false) {
            Ok((dev, queue, caps)) => {
                let swap_chain = dev.create_swap_chain(size.width as u32,
                                                       size.height as u32,
                                                       frame_num as u32);
                let painter = WebMetalPaintThread {
                    device: dev,
                    queue: queue,
                    swap_chain: swap_chain,
                    _size: size,
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
                sender.send(Some(com)).unwrap();
            }
            WebMetalCommand::Submit(com) => {
                self.device.execute(&self.queue, &com);
            }
        }
    }

    /// Creates a new `WebMetalPaintThread` and returns an `IpcSender` to
    /// communicate with it.
    pub fn start(size: Size2D<i32>, frame_num: u8) -> Result<WebMetalInit, String> {
        let (sender, receiver) = ipc::channel::<CanvasMsg>().unwrap();
        let (result_chan, result_port) = channel();
        spawn_named("WebMetalThread".to_owned(), move || {
            let mut painter = match WebMetalPaintThread::new(size, frame_num) {
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
            loop {
                match receiver.recv().unwrap() {
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

    fn send_data(&mut self, _chan: IpcSender<CanvasData>) {
        //WM TODO: actually read back the surface and send it over
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
