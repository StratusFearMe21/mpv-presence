use discord_rich_presence::activity::{Assets, Timestamps};
use discord_rich_presence::{activity, DiscordIpc};
use libmpv_sys::{
    mpv_event_id_MPV_EVENT_END_FILE, mpv_event_id_MPV_EVENT_PLAYBACK_RESTART,
    mpv_event_id_MPV_EVENT_SHUTDOWN, mpv_format_MPV_FORMAT_FLAG, mpv_format_MPV_FORMAT_INT64,
    mpv_format_MPV_FORMAT_NONE, mpv_format_MPV_FORMAT_STRING, mpv_get_property,
    mpv_get_property_string, mpv_handle, mpv_observe_property, mpv_wait_event,
};
use libmpv_sys::{mpv_event_id_MPV_EVENT_PROPERTY_CHANGE, mpv_event_property};
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

#[derive(Debug)]
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
unsafe fn mpv_open_cplugin(mpv: *mut mpv_handle) -> i8 {
    mpv_observe_property(
        mpv,
        0,
        CString::new("pause").unwrap().as_ptr(),
        mpv_format_MPV_FORMAT_FLAG,
    );
    mpv_observe_property(
        mpv,
        0,
        CString::new("media-title").unwrap().as_ptr() as *const i8,
        mpv_format_MPV_FORMAT_STRING,
    );
    mpv_observe_property(
        mpv,
        0,
        CString::new("duration").unwrap().as_ptr() as *const i8,
        mpv_format_MPV_FORMAT_INT64,
    );
    let mut track = MpvTrack::default();
    let mut client = discord_rich_presence::new_client("896460735360679986").unwrap();
    if let Err(e) = client.connect() {
        println!("{}", e.to_string());
        return -1;
    }
    println!("RPC Connected");
    loop {
        let ev = mpv_wait_event(mpv, 600.);
        match *ev {
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
                let dataser = *(data as *mut mpv_event_property);
                if dataser.format != mpv_format_MPV_FORMAT_NONE {
                    let name = CStr::from_ptr(dataser.name).to_str().unwrap();
                    if dataser.format == mpv_format_MPV_FORMAT_FLAG {
                        track.paused = *(dataser.data as *mut bool);
                        if !track.paused {
                            let mut pos_s = 0;
                            mpv_get_property(
                                mpv,
                                CString::new("time-remaining").unwrap().as_ptr(),
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
                        track.title = CStr::from_ptr(*(dataser.data as *mut *mut c_char))
                            .to_str()
                            .unwrap();
                        let artist = mpv_get_property_string(
                            mpv,
                            CString::new("metadata/by-key/Artist").unwrap().as_ptr(),
                        );
                        let album = mpv_get_property_string(
                            mpv,
                            CString::new("metadata/by-key/Album").unwrap().as_ptr(),
                        );
                        if !artist.is_null() {
                            track.artist = CStr::from_ptr(artist).to_str().unwrap();
                        } else {
                            track.artist = "Unknown Artist";
                        }
                        if !album.is_null() {
                            track.album = CStr::from_ptr(album).to_str().unwrap();
                        } else {
                            track.artist = "Unknown Artist";
                        }
                    } else if dataser.format == mpv_format_MPV_FORMAT_INT64 {
                        track.duration = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64
                            + *(dataser.data as *mut i64);
                    }
                }
            }
            libmpv_sys::mpv_event {
                event_id: mpv_event_id_MPV_EVENT_PLAYBACK_RESTART,
                error: 0,
                ..
            } => {
                let mut pos_s = 0;
                mpv_get_property(
                    mpv,
                    CString::new("time-remaining").unwrap().as_ptr(),
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
    return 0;
}
