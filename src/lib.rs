use discord_rich_presence::activity::{Assets, Button, Timestamps};
use discord_rich_presence::{activity, DiscordIpc};
use libmpv_sys::{
    mpv_event_id_MPV_EVENT_PLAYBACK_RESTART, mpv_event_id_MPV_EVENT_SHUTDOWN,
    mpv_format_MPV_FORMAT_FLAG, mpv_format_MPV_FORMAT_INT64, mpv_format_MPV_FORMAT_NONE,
    mpv_format_MPV_FORMAT_STRING, mpv_get_property, mpv_get_property_string, mpv_handle,
    mpv_observe_property, mpv_wait_event,
};
use libmpv_sys::{mpv_event_id_MPV_EVENT_PROPERTY_CHANGE, mpv_event_property};
use std::ffi::c_void;
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

macro_rules! set_activity {
    ($client:expr, $track:expr) => {
        let payload = activity::Activity::new()
            .details($track.title)
            .state(if let Some(artist) = &$track.artist {
                artist.0
            } else {
                "Unknown Artist"
            })
            .timestamps(Timestamps::new().end($track.duration))
            .assets(
                Assets::new()
                    .large_image("mpv")
                    .small_image(if $track.paused {
                        "pause"
                    } else if $track.loop_file {
                        "loop"
                    } else if $track.loop_playlist {
                        "loop-playlist"
                    } else {
                        "play"
                    })
                    .large_text(if let Some(album) = &$track.album {
                        album.0
                    } else {
                        "Unknown Album"
                    })
                    .small_text(if $track.paused {
                        "Paused"
                    } else if $track.loop_file {
                        "Repeat Song"
                    } else if $track.loop_playlist {
                        "Repeat"
                    } else {
                        "Playing"
                    }),
            );
        if $client
            .set_activity(if let Some(path) = &$track.path {
                if path.0.starts_with("http") {
                    payload.buttons(vec![Button::new("Watch Together", path.0)])
                } else {
                    payload
                }
            } else {
                payload
            })
            .is_err()
        {
            $client.reconnect().ok();
        }
    };
}

struct MpvTrack<'a> {
    album: Option<MpvStr<'a>>,
    artist: Option<MpvStr<'a>>,
    path: Option<MpvStr<'a>>,
    title: &'a str,
    duration: i64,
    paused: bool,
    loop_file: bool,
    loop_playlist: bool,
}

impl<'a> Default for MpvTrack<'a> {
    fn default() -> Self {
        Self {
            album: None,
            artist: None,
            path: None,
            title: "Unkown Title",
            duration: 0,
            paused: false,
            loop_playlist: false,
            loop_file: false,
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct MpvStr<'a>(&'a str);
impl<'a> Drop for MpvStr<'a> {
    fn drop(&mut self) {
        unsafe { libmpv_sys::mpv_free(self.0.as_ptr() as *mut u8 as _) };
    }
}

#[no_mangle]
fn mpv_open_cplugin(mpv: *mut mpv_handle) -> i8 {
    observe_property(mpv, 0, "pause", mpv_format_MPV_FORMAT_FLAG);
    observe_property(mpv, 0, "media-title", mpv_format_MPV_FORMAT_STRING);
    observe_property(mpv, 0, "duration", mpv_format_MPV_FORMAT_INT64);
    observe_property(mpv, 0, "loop-file", mpv_format_MPV_FORMAT_STRING);
    observe_property(mpv, 0, "loop-playlist", mpv_format_MPV_FORMAT_STRING);
    let mut track = MpvTrack::default();
    let mut client = discord_rich_presence::new_client("896460735360679986").unwrap();
    if let Err(e) = client.connect() {
        println!("{}", e.to_string());
        return -1;
    }
    println!("RPC Connected");
    loop {
        #[allow(non_upper_case_globals)]
        match unsafe { *mpv_wait_event(mpv, 600.) } {
            libmpv_sys::mpv_event {
                event_id: mpv_event_id_MPV_EVENT_SHUTDOWN,
                error: 0,
                ..
            } => {
                break;
            }
            libmpv_sys::mpv_event {
                event_id: mpv_event_id_MPV_EVENT_PROPERTY_CHANGE,
                error: 0,
                data,
                ..
            } => {
                let dataser = unsafe { *(data as *mut mpv_event_property) };
                if dataser.format != mpv_format_MPV_FORMAT_NONE {
                    match unsafe { CStr::from_ptr(dataser.name).to_str().unwrap() } {
                        "pause" => {
                            track.paused = unsafe { *(dataser.data as *mut bool) };
                            if !track.paused {
                                let mut pos_s = 0;
                                get_property(
                                    mpv,
                                    "time-remaining",
                                    mpv_format_MPV_FORMAT_INT64,
                                    &mut pos_s as *mut i64 as _,
                                );
                                track.duration = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                                    as i64
                                    + pos_s;
                            }
                            set_activity!(client, track);
                        }
                        "media-title" => {
                            track.title = unsafe {
                                CStr::from_ptr(*(dataser.data as *mut *mut c_char))
                                    .to_str()
                                    .unwrap()
                            };
                            let artist = get_property_string(mpv, "metadata/by-key/Artist");
                            let album = get_property_string(mpv, "metadata/by-key/Album");
                            let path = get_property_string(mpv, "path");
                            if !artist.is_null() {
                                track.artist = unsafe {
                                    Some(MpvStr(CStr::from_ptr(artist).to_str().unwrap()))
                                };
                            } else {
                                track.artist = None;
                            }
                            if !album.is_null() {
                                track.album = unsafe {
                                    Some(MpvStr(CStr::from_ptr(album).to_str().unwrap()))
                                };
                            } else {
                                track.album = None;
                            }
                            if !path.is_null() {
                                track.path =
                                    unsafe { Some(MpvStr(CStr::from_ptr(path).to_str().unwrap())) };
                            } else {
                                track.path = None;
                            }
                        }
                        "duration" => {
                            track.duration = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64
                                + unsafe { *(dataser.data as *mut i64) };
                        }
                        "loop-file" => {
                            track.loop_file = matches!(
                                unsafe {
                                    CStr::from_ptr(*(dataser.data as *mut *mut c_char))
                                        .to_str()
                                        .unwrap()
                                },
                                "inf" | "yes"
                            );
                            set_activity!(client, track);
                        }
                        "loop-playlist" => {
                            track.loop_playlist = unsafe {
                                CStr::from_ptr(*(dataser.data as *mut *mut c_char))
                                    .to_str()
                                    .unwrap()
                            } == "yes";
                            set_activity!(client, track);
                        }
                        _ => {}
                    };
                }
            }
            libmpv_sys::mpv_event {
                event_id: mpv_event_id_MPV_EVENT_PLAYBACK_RESTART,
                error: 0,
                ..
            } => {
                let mut pos_s = 0;
                get_property(
                    mpv,
                    "time-remaining",
                    mpv_format_MPV_FORMAT_INT64,
                    &mut pos_s as *mut i64 as _,
                );
                track.duration = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    + pos_s;
                set_activity!(client, track);
            }
            libmpv_sys::mpv_event { .. } => {}
        }
    }
    client.close().unwrap();
    0
}

fn observe_property(handle: *mut mpv_handle, id: u64, name: &str, format: u32) {
    let name = CString::new(name).unwrap();
    unsafe {
        mpv_observe_property(handle, id, name.as_ptr(), format);
    }
}

fn get_property_string(handle: *mut mpv_handle, name: &str) -> *mut i8 {
    let name = CString::new(name).unwrap();
    unsafe { mpv_get_property_string(handle, name.as_ptr()) }
}

fn get_property(handle: *mut mpv_handle, name: &str, format: u32, var: *mut c_void) {
    let name = CString::new(name).unwrap();
    unsafe {
        mpv_get_property(handle, name.as_ptr(), format, var);
    }
}
