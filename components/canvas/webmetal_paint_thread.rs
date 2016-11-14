/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasCommonMsg, CanvasData, CanvasMsg};
use canvas_traits::{FromLayoutMsg, FromScriptMsg};
use euclid::size::Size2D;
use ipc_channel::ipc::{self, IpcSender};
use std::sync::mpsc::channel;
use util::thread::spawn_named;
use webmetal::{Backend, Options, WebMetalCapabilities, WebMetalCommand};

pub struct WebMetalPaintThread {
    _size: Size2D<i32>,
    _backend: Backend,
}

impl WebMetalPaintThread {
    fn new(size: Size2D<i32>)
        -> Result<(WebMetalPaintThread, WebMetalCapabilities), String> {
        let options = Options {
            width: size.width as u16,
            height: size.height as u16,
            debug: false,
        };
        match Backend::new(options) {
            Ok((b, caps)) => {
                let painter = WebMetalPaintThread {
                    _size: size,
                    _backend: b,
                };
                Ok((painter, caps))
            }
            Err(e) => Err(format!("{:?}", e))
        }
    }

    fn init(&mut self) {
        //WM TODO
    }

    fn handle_message(&self, message: WebMetalCommand) {
        debug!("WebMetal message: {:?}", message);
    }

    /// Creates a new `WebMetalPaintThread` and returns an `IpcSender` to
    /// communicate with it.
    pub fn start(size: Size2D<i32>)
                 -> Result<(IpcSender<CanvasMsg>, WebMetalCapabilities), String> {
        let (sender, receiver) = ipc::channel::<CanvasMsg>().unwrap();
        let (result_chan, result_port) = channel();
        spawn_named("WebMetalThread".to_owned(), move || {
            let mut painter = match WebMetalPaintThread::new(size) {
                Ok((thread, caps)) => {
                    result_chan.send(Ok(caps)).unwrap();
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

        result_port.recv().unwrap().map(|limits| (sender, limits))
    }

    fn send_data(&mut self, _chan: IpcSender<CanvasData>) {
        //WM TODO
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
