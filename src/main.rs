use itertools::Itertools;
use js_sys::Date;
use leptos::{
    ev::{MouseEvent, PointerEvent},
    leptos_dom::helpers::location,
    *,
};
use std::{cmp, collections::HashMap};
use url::Url;

pub fn main() {
    let href = location().href().unwrap();
    let url = Url::parse(&href).unwrap();
    let queries = url.query_pairs().into_owned().collect::<HashMap<_, _>>();
    let tokens = queries["q"].split("-").collect::<Vec<_>>();

    let h = tokens[0].parse::<usize>().unwrap();
    let w = tokens[1].parse::<usize>().unwrap();

    let mut lasers = vec![];
    let mut targets = vec![];

    for &token in &tokens[2..] {
        let c = &token[0..1];
        let i = token[1..].parse::<usize>().unwrap();

        let color = match c.to_ascii_uppercase().as_str() {
            "R" => Color::Red,
            "Y" => Color::Yellow,
            "G" => Color::Green,
            "B" => Color::Blue,
            "P" => Color::Purple,
            _ => panic!(),
        };

        let (y, x) = (|| {
            let mut i = i;

            if i < h + 1 {
                return (i as i32, -1);
            }
            i -= h + 1;

            if i < w + 1 {
                return ((h + 1) as i32, i as i32);
            }
            i -= w + 1;

            if i < h + 1 {
                return ((h - i) as i32, (w + 1) as i32);
            }
            i -= h + 1;

            (-1, (w - i) as i32)
        })();

        let p = ColoredPoint { y, x, color };

        if c.to_ascii_uppercase() == c {
            lasers.push(p);
        } else {
            targets.push(p);
        }
    }

    mount_to_body(move |cx| view! { cx, <Main h=h w=w lasers=lasers targets=targets /> });
}

#[derive(Copy, Clone, PartialEq)]
struct Point {
    y: i32,
    x: i32,
}

#[derive(Copy, Clone, PartialEq)]
enum Color {
    Red,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl Color {
    fn fill(&self) -> &'static str {
        match *self {
            Color::Red => "fill-red-500",
            Color::Yellow => "fill-yellow-500",
            Color::Green => "fill-green-500",
            Color::Blue => "fill-blue-500",
            Color::Purple => "fill-purple-500",
        }
    }
}

#[derive(Copy, Clone)]
struct Laser {
    path: Memo<Vec<Point>>,
    color: Color,
}

impl Laser {
    fn new(cx: Scope, a: Vec<Vec<RwSignal<bool>>>, y: i32, x: i32, color: Color) -> Laser {
        let f = move |_: Option<&_>| {
            let h = a.len();
            let w = a[0].len();

            let dy = [0, -1, 0, 1];
            let dx = [-1, 0, 1, 0];

            let mut y = y;
            let mut x = x;
            let mut dir = {
                if x == -1 {
                    2
                } else if y == (h + 1) as i32 {
                    1
                } else if x == (w + 1) as i32 {
                    0
                } else {
                    3
                }
            };

            let mut path = vec![];

            loop {
                if [-2, (h + 2) as i32].contains(&y) || [-2, (w + 2) as i32].contains(&x) {
                    break;
                }

                path.push(Point { y, x });

                let f = |y, x| {
                    (0..h as i32).contains(&y)
                        && (0..w as i32).contains(&x)
                        && a[y as usize][x as usize]()
                };

                let neighbors = [f(y, x - 1), f(y - 1, x - 1), f(y - 1, x), f(y, x)];

                let left = neighbors[dir as usize];
                let right = neighbors[(dir + 1) % 4];

                if left && right {
                    break;
                }
                if left && !right {
                    dir = (dir + 1) % 4;
                }
                if !left && right {
                    dir = (dir + 3) % 4;
                }

                y += dy[dir];
                x += dx[dir];
            }

            path
        };

        Laser {
            path: create_memo(cx, f),
            color,
        }
    }
}

#[derive(Copy, Clone)]
struct Target {
    y: i32,
    x: i32,
    color: Color,
    hit: Memo<bool>,
}

impl Target {
    fn new(cx: Scope, lasers: Vec<Laser>, y: i32, x: i32, color: Color) -> Target {
        let f = move |_: Option<&_>| {
            lasers.iter().any(|laser| {
                let path = (laser.path)();
                let p = path.last().unwrap();
                (p.y, p.x, laser.color) == (y, x, color)
            })
        };

        Target {
            y,
            x,
            color,
            hit: create_memo(cx, f),
        }
    }
}

struct ColoredPoint {
    y: i32,
    x: i32,
    color: Color,
}

#[component]
fn Main(
    cx: Scope,
    h: usize,
    w: usize,
    lasers: Vec<ColoredPoint>,
    targets: Vec<ColoredPoint>,
) -> impl IntoView {
    let a = (0..h)
        .map(|_| {
            (0..w)
                .map(|_| create_rw_signal(cx, false))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let lasers = lasers
        .iter()
        .map(|p| Laser::new(cx, a.clone(), p.y, p.x, p.color))
        .collect::<Vec<_>>();
    let targets = targets
        .iter()
        .map(|p| Target::new(cx, lasers.clone(), p.y, p.x, p.color))
        .collect::<Vec<_>>();

    let elapsed_seconds = create_rw_signal(cx, 0.0);
    let initial_epoch = Date::now() / 1000.0;
    let on_animationframe = move |_| elapsed_seconds.set(Date::now() / 1000.0 - initial_epoch);
    window_event_listener("animationframe", on_animationframe);

    let solve_seconds = {
        let targets = targets.clone();

        create_memo(cx, move |prev| match prev {
            Some(&Some(solve_seconds)) => Some(solve_seconds),
            _ => {
                if targets.iter().all(|target| (target.hit)()) {
                    Some(Date::now() / 1000.0 - initial_epoch)
                } else {
                    None
                }
            }
        })
    };

    view! { cx,
        <div class="container mx-auto flex flex-col items-center">
            <svg xmlns="http://www.w3.org/2000/svg" width=(w + 3) * 64 height=(h + 3) * 64 viewBox=format!("-8 -8 {} {}", (w + 3) * 16, (h + 3) * 16) class="mx-auto">
                <rect x=-8 y=-8 width="100%" height="100%" class="fill-slate-900" />
                {
                    let a = a.clone();

                    move || {
                        a.iter().enumerate().flat_map(move |(i, row)|
                            row.iter().enumerate().filter_map(move |(j, cell)| {
                                    if cell() {
                                        Some(
                                            view! { cx,
                                                <rect x=(1 + j) * 16 y=(1 + i) * 16 width=16 height=16 class="fill-slate-500" />
                                            }
                                        )
                                    } else {
                                        None
                                    }
                                }
                            )
                        ).collect::<Vec<_>>()
                    }
                }
                {
                    let lasers = lasers.clone();

                    move || {
                        lasers.iter().map(|laser|
                            view! { cx,
                                <Laser laser=*laser />
                            }
                        ).collect::<Vec<_>>()
                    }
                }
                {
                    let targets = targets.clone();

                    move || {
                        targets.iter().map(|target|
                            view! { cx,
                                <Target target=*target />
                            }
                        ).collect::<Vec<_>>()
                    }
                }
                {
                    move || {
                        (0..(h + 1)).flat_map(move |i|
                            (0..(w + 1)).map(move |j|
                                view! { cx,
                                    <rect x=(1 + j) * 16 - 1 y=(1 + i) * 16 - 1 width=2 height=2 class="fill-slate-50" fill-opacity=0.2 />
                                }
                            )
                        ).collect::<Vec<_>>()
                    }
                }
                {
                    let a = a.clone();

                    move || {
                        a.iter().enumerate().flat_map(move |(i, row)|
                            row.iter().enumerate().map(move |(j, cell)|
                                view! { cx,
                                    <g transform=format!("translate({} {}) scale({} {})", (1 + j) * 16, (1 + i) * 16, 16, 16)>
                                        <Pad x=*cell />
                                    </g>
                                }
                            )
                        ).collect::<Vec<_>>()
                    }
                }
            </svg>
            <div class="h-8" />
            <Timer elapsed_seconds=elapsed_seconds solve_seconds />
        </div>
    }
}

#[component]
fn Pad(cx: Scope, x: RwSignal<bool>) -> impl IntoView {
    let set = move |e: PointerEvent| match e.buttons() & 3 {
        1 => x.set(true),
        2 => x.set(false),
        _ => (),
    };

    let on_pointerenter = move |e: PointerEvent| {
        set(e);
    };

    let on_pointerdown = move |e: PointerEvent| {
        set(e);
    };

    let on_contextmenu = move |e: MouseEvent| {
        e.prevent_default();
    };

    view! { cx,
        <rect x=0 y=0 width=1 height=1 fill="none" pointer-events="fill" on:pointerenter=on_pointerenter on:pointerdown=on_pointerdown on:contextmenu=on_contextmenu />
    }
}

#[component]
fn Laser(cx: Scope, laser: Laser) -> impl IntoView {
    move || {
        view! { cx,
            {
                (laser.path)().iter().tuple_windows().map(|(p, q)| {
                    let yl = cmp::min(p.y, q.y);
                    let yr = cmp::max(p.y, q.y);
                    let xl = cmp::min(p.x, q.x);
                    let xr = cmp::max(p.x, q.x);

                    view! { cx,
                        <rect x=(1 + xl) * 16 - 1 y=(1 + yl) * 16 - 1 width=(xr - xl) * 16 + 2 height=(yr - yl) * 16 + 2 class=laser.color.fill() />
                    }
                }).collect::<Vec<_>>()
            }
            {
                let path = (laser.path)();
                let p = path.first().unwrap();

                view! { cx,
                    <rect x=(1 + p.x) * 16 - 4 y=(1 + p.y) * 16 - 4 width=8 height=8 class=laser.color.fill() />
                }
            }
        }
    }
}

#[component]
fn Target(cx: Scope, target: Target) -> impl IntoView {
    move || {
        view! { cx,
            <rect x=(1 + target.x) * 16 - 6 y=(1 + target.y) * 16 - 6 width=12 height=12 class=target.color.fill() />
            <rect x=(1 + target.x) * 16 - 4 y=(1 + target.y) * 16 - 4 width=8 height=8 class="fill-slate-900" />
            <Show when=move || (target.hit)() fallback=|_| ()>
                <svg x=((1 + target.x) * 16) as f64 - 3.5 y=((1 + target.y) * 16) as f64 - 3.5 width=7 height=7 viewBox="0 0 512 512" class=target.color.fill()>
                    <path d="M470.6 105.4c12.5 12.5 12.5 32.8 0 45.3l-256 256c-12.5 12.5-32.8 12.5-45.3 0l-128-128c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L192 338.7 425.4 105.4c12.5-12.5 32.8-12.5 45.3 0z" />
                </svg>
            </Show>
        }
    }
}

#[component]
fn Timer(
    cx: Scope,
    elapsed_seconds: RwSignal<f64>,
    solve_seconds: Memo<Option<f64>>,
) -> impl IntoView {
    move || match solve_seconds() {
        Some(solve_seconds) => {
            let m = (solve_seconds as u32) / 60;
            let s = (solve_seconds as u32) % 60;

            view! { cx,
                <div class="font-mono text-2xl text-green-500" >{ format!("{:02}:{:02}", m, s) }</div>
            }
        }

        None => {
            let m = (elapsed_seconds() as u32) / 60;
            let s = (elapsed_seconds() as u32) % 60;

            view! { cx,
                <div class="font-mono text-2xl" >{ format!("{:02}:{:02}", m, s) }</div>
            }
        }
    }
}
