/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use WindowWrapper;
use blob;
use std::sync::mpsc::{channel, Receiver, Sender};
use webrender::api::*;
use wrench::Wrench;

pub struct RawtestHarness<'a> {
    wrench: &'a mut Wrench,
    rx: Receiver<()>,
    window: &'a mut WindowWrapper,
}

impl<'a> RawtestHarness<'a> {
    pub fn new(wrench: &'a mut Wrench,
               window: &'a mut WindowWrapper) -> RawtestHarness<'a>
    {
        // setup a notifier so we can wait for frames to be finished
        struct Notifier {
            tx: Sender<()>,
        };
        impl RenderNotifier for Notifier {
            fn new_frame_ready(&mut self) { self.tx.send(()).unwrap(); }
            fn new_scroll_frame_ready(&mut self, _composite_needed: bool) {}
        }
        let (tx, rx) = channel();
        wrench.renderer.set_render_notifier(Box::new(Notifier { tx: tx }));

        RawtestHarness {
            wrench: wrench,
            rx: rx,
            window: window,
        }
    }

    pub fn run(mut self) {
        self.retained_blob_images_test();
    }

    fn render_and_get_pixels(&mut self, window_rect: DeviceUintRect) -> Vec<u8> {
        self.rx.recv().unwrap();
        self.wrench.render();
        self.wrench.renderer.read_pixels_rgba8(window_rect)
    }

    fn retained_blob_images_test(&mut self) {
        let blob_img;
        let window_size = self.window.get_inner_size_pixels();
        let window_size = DeviceUintSize::new(window_size.0, window_size.1);

        let test_size = DeviceUintSize::new(400, 400);
        let document_id = self.wrench.document_id;

        let window_rect = DeviceUintRect::new(DeviceUintPoint::new(0, window_size.height - test_size.height),
                                              test_size);
        let layout_size = LayoutSize::new(400., 400.);
        let mut resources = ResourceUpdates::new();
        {
            let api = &self.wrench.api;

            blob_img = api.generate_image_key();
            resources.add_image(blob_img,
                          ImageDescriptor::new(500, 500, ImageFormat::BGRA8, true),
                          ImageData::new_blob_image(blob::serialize_blob(ColorU::new(50, 50, 150, 255))),
                          None,
            );
        }
        let root_background_color = Some(ColorF::new(1.0, 1.0, 1.0, 1.0));

        // draw the blob the first time
        let mut builder = DisplayListBuilder::new(self.wrench.root_pipeline_id, layout_size);
        builder.push_image(
            LayoutRect::new(LayoutPoint::new(0.0, 60.0), LayoutSize::new(200.0, 200.0)),
            None,
            LayoutSize::new(200.0, 200.0),
            LayoutSize::new(0.0, 0.0),
            ImageRendering::Auto,
            blob_img,
        );

        self.wrench.api.set_display_list(document_id,
                                         Epoch(0),
                                         root_background_color,
                                         layout_size,
                                         builder.finalize(),
                                         false,
                                         resources);
        self.wrench.api.generate_frame(document_id, None);


        // draw the blob image a second time at a different location

        // make a new display list that refers to the first image
        let mut builder = DisplayListBuilder::new(self.wrench.root_pipeline_id, layout_size);
        builder.push_image(
            LayoutRect::new(LayoutPoint::new(1.0, 60.0), LayoutSize::new(200.0, 200.0)),
            None,
            LayoutSize::new(200.0, 200.0),
            LayoutSize::new(0.0, 0.0),
            ImageRendering::Auto,
            blob_img,
        );

        self.wrench.api.set_display_list(document_id,
                                         Epoch(1),
                                         root_background_color,
                                         layout_size,
                                         builder.finalize(),
                                         false,
                                         ResourceUpdates::new());

        self.wrench.api.generate_frame(document_id, None);

        let pixels_first = self.render_and_get_pixels(window_rect);
        let pixels_second = self.render_and_get_pixels(window_rect);

        // use png;
        // png::save_flipped("out1.png", &pixels_first, window_rect.size);
        // png::save_flipped("out2.png", &pixels_second, window_rect.size);

        assert!(pixels_first != pixels_second);

    }
}
