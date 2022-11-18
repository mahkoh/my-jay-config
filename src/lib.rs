use {
    chrono::{format::StrftimeItems, Local},
    jay_config::{
        config,
        exec::{set_env, Command},
        get_workspace,
        input::{
            capability::CAP_POINTER, get_seat, input_devices, on_new_input_device, InputDevice,
            Seat,
        },
        keyboard::{
            mods::{Modifiers, ALT, CTRL, SHIFT},
            parse_keymap,
            syms::{
                SYM_Super_L, SYM_a, SYM_c, SYM_d, SYM_e, SYM_f, SYM_h, SYM_i, SYM_j, SYM_k, SYM_l,
                SYM_m, SYM_n, SYM_o, SYM_p, SYM_q, SYM_r, SYM_t, SYM_u, SYM_v, SYM_y, SYM_F1,
                SYM_F10, SYM_F11, SYM_F12, SYM_F13, SYM_F14, SYM_F15, SYM_F16, SYM_F17, SYM_F18,
                SYM_F19, SYM_F2, SYM_F20, SYM_F21, SYM_F22, SYM_F23, SYM_F24, SYM_F25, SYM_F3,
                SYM_F4, SYM_F5, SYM_F6, SYM_F7, SYM_F8, SYM_F9,
            },
        },
        on_idle, quit, reload,
        status::set_status,
        switch_to_vt,
        timer::{duration_until_wall_clock_is_multiple_of, get_timer},
        video::{get_connector, on_connector_connected, on_graphics_initialized, on_new_connector},
        Axis::{Horizontal, Vertical},
        Direction::{Down, Left, Right, Up},
    },
    std::{
        cell::{Cell, RefCell},
        time::Duration,
    },
    sysinfo::{CpuExt, CpuRefreshKind, RefreshKind, System, SystemExt},
};

const MOD: Modifiers = ALT;

fn configure_seat(s: Seat) {
    s.bind(MOD | SYM_h, move || s.focus(Left));
    s.bind(MOD | SYM_j, move || s.focus(Down));
    s.bind(MOD | SYM_k, move || s.focus(Up));
    s.bind(MOD | SYM_l, move || s.focus(Right));

    s.bind(MOD | SHIFT | SYM_h, move || s.move_(Left));
    s.bind(MOD | SHIFT | SYM_j, move || s.move_(Down));
    s.bind(MOD | SHIFT | SYM_k, move || s.move_(Up));
    s.bind(MOD | SHIFT | SYM_l, move || s.move_(Right));

    s.bind(MOD | SYM_d, move || s.create_split(Horizontal));
    s.bind(MOD | SYM_v, move || s.create_split(Vertical));

    s.bind(MOD | SYM_t, move || s.toggle_split());
    s.bind(MOD | SYM_m, move || s.toggle_mono());
    s.bind(MOD | SYM_u, move || s.toggle_fullscreen());
    s.bind(MOD | SHIFT | SYM_f, move || s.toggle_floating());
    s.bind(MOD | SYM_f, move || s.focus_parent());

    s.bind(MOD | SHIFT | SYM_c, move || s.close());

    let alacritty = || Command::new("alacritty").spawn();
    s.bind(SYM_Super_L, alacritty);
    // s.bind(MOD | SYM_Return, alacritty);

    s.bind(MOD | SYM_p, || Command::new("bemenu-run").spawn());

    s.bind(MOD | SYM_q, quit);
    s.bind(MOD | SHIFT | SYM_r, reload);

    s.bind(MOD | SYM_n, move || s.disable_pointer_constraint());

    const STEP: f64 = 64.0 / 256.0;
    let delta = |dist| {
        let c = get_connector("HDMI-A-1");
        c.set_scale(c.scale() + dist);
        arrange_outputs();
    };
    s.bind(MOD | SYM_y, move || delta(STEP));
    s.bind(MOD | SYM_i, move || delta(-STEP));

    let use_hc = Cell::new(true);
    s.bind(MOD | SHIFT | SYM_m, move || {
        let hc = !use_hc.get();
        use_hc.set(hc);
        log::info!("use hc = {}", hc);
        s.use_hardware_cursor(hc);
    });

    let fnkeys = [
        SYM_F1, SYM_F2, SYM_F3, SYM_F4, SYM_F5, SYM_F6, SYM_F7, SYM_F8, SYM_F9, SYM_F10, SYM_F11,
        SYM_F12,
    ];
    for (i, sym) in fnkeys.into_iter().enumerate() {
        s.bind(CTRL | ALT | sym, move || switch_to_vt(i as u32 + 1));
    }

    let fnkeys2 = [
        SYM_F13, SYM_F14, SYM_F15, SYM_F16, SYM_F17, SYM_F18, SYM_F19, SYM_F20, SYM_F21, SYM_F22,
        SYM_F23, SYM_F24, SYM_F25,
    ];
    for (i, sym) in fnkeys2.into_iter().enumerate() {
        let ws = get_workspace(&format!("{}", i + 1));
        s.bind(MOD | sym, move || s.show_workspace(ws));
        s.bind(MOD | SHIFT | sym, move || s.set_workspace(ws));
    }

    let spotify = |sym, arg| {
        s.bind(MOD | sym, move || {
            Command::new("spotify-remote").arg(arg).spawn()
        })
    };
    spotify(SYM_a, "a");
    spotify(SYM_o, "o");
    spotify(SYM_e, "e");
}

fn setup_seats() {
    let seat = get_seat("default");
    seat.set_keymap(parse_keymap(include_str!("keymap.xkb")));
    configure_seat(seat);
    let handle_input_device = move |device: InputDevice| {
        if device.has_capability(CAP_POINTER) {
            device.set_left_handed(true);
            device.set_transform_matrix([[0.35, 0.0], [0.0, 0.35]]);
        }
        device.set_tap_enabled(true);
        device.set_seat(seat);
    };
    input_devices().into_iter().for_each(handle_input_device);
    on_new_input_device(handle_input_device);
}

fn arrange_outputs() {
    let left = get_connector("HDMI-A-1");
    let right = get_connector("DP-3");
    if left.connected() && right.connected() {
        left.set_position(0, 0);
        right.set_position(left.width(), 0);
    }
}

fn setup_outputs() {
    on_new_connector(move |_| arrange_outputs());
    on_connector_connected(move |_| arrange_outputs());
    arrange_outputs();
}

fn setup_status() {
    let time_format: Vec<_> = StrftimeItems::new("%Y-%m-%d %H:%M:%S").collect();
    let specifics = RefreshKind::new()
        .with_cpu(CpuRefreshKind::new().with_cpu_usage())
        .with_memory();
    let system = RefCell::new(System::new_with_specifics(specifics));
    let update_status = move || {
        let mut system = system.borrow_mut();
        system.refresh_specifics(specifics);
        let cpu_usage = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / 100.0;
        let used = system.used_memory() as f64 / (1024 * 1024) as f64;
        let total = system.total_memory() as f64 / (1024 * 1024) as f64;
        let status = format!(
            r##"MEM: {:.1}/{:.1} <span color="#333333">|</span> CPU: {:5.2} <span color="#333333">|</span> {}"##,
            used,
            total,
            cpu_usage,
            Local::now().format_with_items(time_format.iter())
        );
        set_status(&status);
    };
    update_status();
    let period = Duration::from_secs(5);
    let timer = get_timer("status_timer");
    timer.repeated(duration_until_wall_clock_is_multiple_of(period), period);
    timer.on_tick(update_status);
}

fn configure() {
    setup_seats();
    setup_outputs();
    setup_status();

    set_env("GTK_THEME", "Adwaita:dark");

    on_graphics_initialized(|| {
        Command::new("mako").spawn();
    });

    on_idle(|| {
        Command::new("jay")
            .arg("run-privileged")
            .arg("--")
            .arg("swaylock")
            .arg("-c")
            .arg("111111")
            .spawn()
    })
}

config!(configure);
