use iced::{
    executor, widget::{column, container, row}, Command, Element, Length, Renderer, Theme, Subscription, advanced::Application, time
};
use std::time::Duration;
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use iced::widget::image as img;
use vanilla_iced::Size;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;
fn main() {
    App::run(Default::default()).unwrap();
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Ticking {
        last_tick: Instant,
    },
}


#[derive(Clone, Debug)]
enum Message {
    Tick(Instant),
}

pub struct Video {
    pub view: vanilla_iced::Program,
    pub wait: mpsc::Receiver<vanilla_iced::Yuv>,
}


struct App {
    pub videos: Vec<Video>,
}

impl Video {
    
    pub fn new(uri: &str) -> Video {
        gst::init().unwrap();
        let source: gst::Element = gst::parse::launch(&format!("v4l2src device={} ! capsfilter caps=video/x-raw,width=720,height=480,framerate=30/1,pixelformat=YUVY ! videoconvert ! appsink name=app_sink caps=video/x-raw,width=720,height=480,format=I420",uri)).unwrap();
        println!("source = {source:#?}");
        //let source = source.downcast::<gst::Bin>().unwrap();
        //println!("source = {source:#?}");
        let source = source.downcast::<gst::Bin>().unwrap();
        let app_sink = source.by_name("app_sink").unwrap();
        let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();

        source.set_state(gst::State::Playing).unwrap();

        // wait for up to 5 seconds until the decoder gets the source capabilities
        source.state(gst::ClockTime::from_seconds(5)).0.unwrap();

        // extract resolution and framerate
        // TODO(jazzfool): maybe we want to extract some other information too?
        let width = 720;//s.get::<i32>("width").map_err(|_| Error::Caps)?;
        let height = 480;//s.get::<i32>("height").map_err(|_| Error::Caps)?;
        
        let (notify, wait) = mpsc::channel();

        app_sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
                    let yuv = vanilla_iced::Yuv {
                        format: vanilla_iced::Format::I420, // yuv format
                        data: map.as_slice().to_vec(), // raw yuv data
                        dimensions: Size { width: width, height: height }
                    };
                    
                    notify.send(yuv).map_err(|_| gst::FlowError::Error)?;

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
        
        Video { 
            view: vanilla_iced::Program::new(vanilla_iced::Yuv {
                format: vanilla_iced::Format::I420, // yuv format
                data: vec![0; (width * height * 4) as _], // raw yuv data
                dimensions: Size { width: width, height: height }
            }),
            wait 
        }
    }

}

impl Application for App {
    type Message = Message;
    type Executor = executor::Default;
    type Renderer = iced::Renderer;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags : ()) -> (App, Command<Self::Message>) {


        (App {
            videos : vec![Video::new("/dev/video3")]
        }, Command::none())
    }

    fn title(&self) -> String {
        String::from("Video Player")
    }

    
    fn update(&mut self, message: Message) -> iced::Command<Message> {
        for video in &mut self.videos {
            if let Ok(yuv) = video.wait.try_recv() {
                video.view.update_frame(yuv);
            }
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(Duration::from_millis(1)).map(Message::Tick);


        Subscription::batch(vec![tick])
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Self::Renderer> {

        container(
            column![
                row![
                    iced::widget::shader(&self.videos[0].view)
                    .width(Length::Fill)
                    .height(Length::Fill)
                ]
                .width(Length::Fill)
                .height(Length::Fill),
            ]
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}