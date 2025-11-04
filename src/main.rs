use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fs, io, thread};

use cursive::traits::*;
use cursive::views::{Dialog, LinearLayout, OnEventView, ResizedView, ScrollView, SelectView};
use cursive::{Cursive, CursiveRunnable};

use rodio::{OutputStreamBuilder, Sink};

const MUSIC_FOLDER: &str = "music";

#[allow(dead_code)]
struct AudioData {
    stream: rodio::OutputStream,
    sink: Arc<Mutex<Sink>>,
}

fn main() -> io::Result<()> {
    let mut app = create_app()?;
    app.run();
    Ok(())
}

fn create_app() -> Result<CursiveRunnable, io::Error> {
    let mut siv = cursive::default();
    siv.add_global_callback('q', |s| s.quit());

    siv.add_global_callback('p', |s| {
        s.with_user_data(|audio: &mut AudioData| {
            audio.sink.lock().unwrap().pause();
        });
    });
    siv.set_theme(cursive::theme::Theme::terminal_default());

    let layout = LinearLayout::horizontal()
        .child(create_list("Artists", load_files(None)?))
        .child(create_list("Albums", vec![]))
        .child(create_list("Songs", vec![]))
        .with_name("Layout")
        .full_screen();

    siv.add_fullscreen_layer(layout);

    Ok(siv)
}

fn create_list(title: &str, contents: Vec<String>) -> ResizedView<OnEventView<Dialog>> {
    let select = SelectView::new()
        .with_all_str(contents)
        .on_submit(select_item)
        .with_name(title);

    let dialog = Dialog::around(ScrollView::new(select)).title(title);

    OnEventView::new(dialog)
        .on_event(cursive::event::Key::Backspace, clear_list)
        .full_screen()
}

fn select_item(siv: &mut Cursive, choice: &String) {
    let focus = siv
        .call_on_name("Layout", |view: &mut LinearLayout| view.get_focus_index())
        .unwrap();

    match focus {
        0 => {
            siv.call_on_name("Albums", |view: &mut SelectView<String>| {
                let contents = load_files(Some(choice.clone())).unwrap();
                view.clear();
                view.add_all_str(contents);
            });
            siv.call_on_name("Layout", |view: &mut LinearLayout| view.set_focus_index(1));
        }
        1 => {
            let prev_choice = siv
                .call_on_name("Artists", |view: &mut SelectView<String>| -> String {
                    let id = view.selected_id().unwrap();
                    let item = view.get_item(id).unwrap();
                    item.1.to_string()
                })
                .unwrap();

            let full_choice = Some(prev_choice + "/" + choice);

            siv.call_on_name("Songs", |view: &mut SelectView<String>| {
                view.clear();
                view.add_all_str(load_files(full_choice).unwrap());
            });

            siv.call_on_name("Layout", |view: &mut LinearLayout| view.set_focus_index(2));
        }
        2 => {
            let artist = siv
                .call_on_name("Artists", |view: &mut SelectView<String>| {
                    let id = view.selected_id().unwrap();
                    let item = view.get_item(id).unwrap();
                    item.1.to_string()
                })
                .unwrap();
            let album = siv
                .call_on_name("Albums", |view: &mut SelectView<String>| {
                    let id = view.selected_id().unwrap();
                    let item = view.get_item(id).unwrap();
                    item.1.to_string()
                })
                .unwrap();

            let track = artist + "/" + &album + "/" + choice;
            let mut path = env::home_dir().unwrap();
            path.push(MUSIC_FOLDER.to_string());
            path.push(&track);

            let stream = OutputStreamBuilder::open_default_stream().unwrap();
            let sink = Arc::new(Mutex::new(Sink::connect_new(stream.mixer())));

            siv.set_user_data(AudioData {
                stream: stream,
                sink: sink.clone(),
            });

            let sink_clone = sink.clone();

            thread::spawn(move || {
                let file = std::fs::File::open(path).unwrap();

                sink_clone
                    .lock()
                    .unwrap()
                    .append(rodio::Decoder::try_from(file).unwrap());
            });
        }
        _ => {}
    }
}

fn clear_list(siv: &mut Cursive) {
    let focus = siv
        .call_on_name("Layout", |view: &mut LinearLayout| view.get_focus_index())
        .unwrap();

    match focus {
        1 => {
            siv.call_on_name("Albums", |view: &mut SelectView<String>| view.clear());
            siv.call_on_name("Layout", |view: &mut LinearLayout| view.set_focus_index(0));
        }
        2 => {
            siv.call_on_name("Songs", |view: &mut SelectView<String>| view.clear());
            siv.call_on_name("Layout", |view: &mut LinearLayout| view.set_focus_index(1));
        }
        _ => {}
    }
}

fn load_files(folder: Option<String>) -> Result<Vec<String>, io::Error> {
    let mut path = env::home_dir().unwrap();
    path.push(MUSIC_FOLDER.to_string());
    path.push(folder.unwrap_or(String::new()));

    let visible = |p: &Path| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|name| !name.starts_with('.'))
            .unwrap()
    };

    let entries = fs::read_dir(path)?
        .filter_map(|res| res.ok())
        .map(|e| e.path())
        .filter(|p| visible(p))
        .collect::<Vec<PathBuf>>();

    let mut list = entries
        .iter()
        .filter_map(|path| path.file_name()?.to_str())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    list.sort();
    Ok(list)
}
