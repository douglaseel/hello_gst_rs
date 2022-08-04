use gst::prelude::*;
use std::{
    io::{self, BufRead},
    collections::HashMap
};
use std::sync::{Arc, Weak, Mutex};


struct SafeRef<T> {
    reference: Arc<Mutex<T>>,
}

impl<T> SafeRef<T> {
    fn new(from: T) -> SafeRef<T> {
        SafeRef {
            reference: Arc::new(Mutex::new(from)),
        }
    }

    pub fn downgrade(&self) ->  Weak<Mutex<T>> {
        Arc::downgrade(&self.reference)
    }
}


pub trait PipelineObserver {
    fn on_eos(&mut self, ssrc: u32);
}

struct Pipeline {
    pipeline: gst::Pipeline,
    sink: gst::Element
}

impl Pipeline {
    fn new() -> Pipeline {
        let pipeline = gst::Pipeline::new(None);
        pipeline.set_property("async-handling", true);

        let src = gst::ElementFactory::make("reqwesthttpsrc", None).unwrap();
        src.set_property("location", "https://downloads.around.video/sound-assets/clapping-2.ogg");
        src.set_property("is-live", true);

        let demux = gst::ElementFactory::make("oggdemux", None).unwrap();
        let payloader = gst::ElementFactory::make("rtpopuspay", None).unwrap();

        payloader.set_property("pt", 100 as u32);
        payloader.set_property("ssrc", 123456 as u32);

        let queue = gst::ElementFactory::make("queue", None).unwrap();
        let sink = gst::ElementFactory::make("udpsink", None).unwrap();

        sink.set_property("host", "127.0.0.1");
        sink.set_property("port", 50000 as i32);

        pipeline
            .add_many(&[&src, &demux, &payloader, &queue, &sink])
            .unwrap();
        gst::Element::link_many(&[&src, &demux]).unwrap();
        gst::Element::link_many(&[&payloader, &queue, &sink]).unwrap();

        /*
        * We are using a probe to detect EOS events.
        * To use bus, we will need to add glib main loop...
        */
        let payloader_weak = payloader.downgrade();
        demux.connect_pad_added(move |element, _| {
            let payloader = payloader_weak.upgrade().unwrap();

            gst::Element::link_many(&[element, &payloader]).unwrap();

            payloader.set_state(gst::State::Playing).unwrap();
        });

        //pipeline.set_state(gst::State::Playing).unwrap();
        //hash_map.lock().unwrap().insert(100, 100);
        Pipeline {
            pipeline,
            sink,
        }
    }

    fn start(&mut self, observer: Weak<Mutex<dyn PipelineObserver + Send>>) {
        let sink_pad = self.sink.static_pad("sink").unwrap();
        sink_pad.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, probe_info| {
            match probe_info.data {
                Some(gst::PadProbeData::Event(ref ev)) if ev.type_() == gst::EventType::Eos => {
                    let observer = observer.upgrade().unwrap();
                    observer.lock().unwrap().on_eos(123456);
                }
                _ => (),
            };

            gst::PadProbeReturn::Ok
        });
        self.pipeline.set_state(gst::State::Playing).unwrap();
    }
}

struct Audioshare {
    pub hash_map: HashMap<u32, u32>
}

impl Audioshare {
    fn new() -> Audioshare {
        Audioshare {
            hash_map: HashMap::new()
        }
    }

    pub fn add_pipeline(audioshare: &SafeRef<Audioshare>) {
        let mut pipeline = Pipeline::new();

        let audioshare_weak = audioshare.downgrade();

        pipeline.start(audioshare_weak);

        let audioshare_weak = audioshare.downgrade();
        let audioshare = audioshare_weak.upgrade().unwrap();

        audioshare.lock().unwrap().hash_map.insert(123456, 123456);
    }
}

impl PipelineObserver for Audioshare {
    fn on_eos(&mut self, ssrc: u32) {
        println!("EOS => BEFORE {} {}", self.hash_map.len(), ssrc);
        self.hash_map.remove(&123456);
        println!("EOS => AFTER {} {}", self.hash_map.len(), ssrc);

    }
}

fn main() {
    println!("Hello, world!");

    gst::init().expect("GStreamer initialization failed!");

    let audioshare = SafeRef::new(
        Audioshare::new()
    );

    Audioshare::add_pipeline(&audioshare);
   
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        match line {
            Ok(_) => {
                
            }
            Err(_) => break, // with ^Z
        }
    }
}