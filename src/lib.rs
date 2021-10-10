use discord_rich_presence::activity::{Assets, Timestamps};
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

struct MpvTrack<'a> {
    album: &'a str,
    artist: &'a str,
    title: &'a str,
    duration: i64,
    paused: bool,
}

impl<'a> Default for MpvTrack<'a> {
    fn default() -> Self {
        Self {
            album: "Unknown Album",
            artist: "Unknown Artist",
            title: "Unkown Title",
            duration: 0,
            paused: false,
        }
    }
}

#[no_mangle]
fn mpv_open_cplugin(mpv: *mut mpv_handle) -> i8 {
    observe_property(mpv, 0, "pause", mpv_format_MPV_FORMAT_FLAG);
    observe_property(mpv, 0, "media-title", mpv_format_MPV_FORMAT_STRING);
    observe_property(mpv, 0, "duration", mpv_format_MPV_FORMAT_INT64);
    let mut track = MpvTrack::default();
    let mut client = discord_rich_presence::new_client("896460735360679986").unwrap();
    if let Err(e) = client.connect() {
        println!("{}", e.to_string());
        return -1;
    }
    println!("RPC Connected");
    loop {
        let ev = unsafe { *mpv_wait_event(mpv, 600.) };

        #[allow(non_upper_case_globals)]
        match ev {
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
                    if dataser.format == mpv_format_MPV_FORMAT_FLAG {
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
                                .as_secs() as i64
                                + pos_s;
                        }
                        let payload = activity::Activity::new()
                            .details(track.title)
                            .state(track.artist)
                            .timestamps(Timestamps::new().end(track.duration))
                            .assets(
                                Assets::new()
                                    .large_image("mpv")
                                    .small_image(if track.paused { "pause" } else { "play" })
                                    .large_text(track.album),
                            );
                        client.set_activity(payload).unwrap();
                    } else if dataser.format == mpv_format_MPV_FORMAT_STRING {
                        track.title = unsafe {
                            CStr::from_ptr(*(dataser.data as *mut *mut c_char))
                                .to_str()
                                .unwrap()
                        };
                        let artist = get_property_string(mpv, "metadata/by-key/Artist");
                        let album = get_property_string(mpv, "metadata/by-key/Album");
                        if !artist.is_null() {
                            track.artist = unsafe { CStr::from_ptr(artist).to_str().unwrap() };
                        } else {
                            track.artist = "Unknown Artist";
                        }
                        if !album.is_null() {
                            track.album = unsafe { CStr::from_ptr(album).to_str().unwrap() };
                        } else {
                            track.artist = "Unknown Artist";
                        }
                    } else if dataser.format == mpv_format_MPV_FORMAT_INT64 {
                        track.duration = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64
                            + unsafe { *(dataser.data as *mut i64) };
                    }
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
                let payload = activity::Activity::new()
                    .details(track.title)
                    .state(track.artist)
                    .timestamps(Timestamps::new().end(track.duration))
                    .assets(
                        Assets::new()
                            .large_image("mpv")
                            .small_image(if track.paused { "pause" } else { "play" })
                            .large_text(track.album),
                    );
                client.set_activity(payload).unwrap();
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
