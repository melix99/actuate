<div align="center">
  <h1>Actuate</h1>
  <a href="https://crates.io/crates/actuate">
    <img src="https://img.shields.io/crates/v/actuate?style=flat-square"
    alt="Crates.io version" />
  </a>
  <a href="https://docs.rs/actuate">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
   <a href="https://github.com/actuate-rs/actuate/actions">
    <img src="https://github.com/actuate-rs/actuate/actions/workflows/ci.yml/badge.svg"
      alt="CI status" />
  </a>
  <a href="https://discord.gg/AbyAdew3">
    <img src="https://img.shields.io/discord/1306713440873877576.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2" />
</div>

<div align="center">
 <a href="https://github.com/actuate-rs/actuate/tree/main/examples">Examples</a>
</div>

<br />

A high-performance reactive user-interface framework for Rust.
This crate provides a generic library that lets you define UI using declarative, borrow-checker friendly syntax.

## Features

- Efficient and borrow-checker friendly state management: Manage state with components and hooks, all using zero-cost smart pointers
- High-performance multi-platform rendering with [Vello](https://github.com/linebender/vello)
- CSS Block, Flex, and Grid layout support with [Taffy](https://github.com/DioxusLabs/taffy)
- Built-in accessibility via [Accesskit](https://github.com/AccessKit/accesskit)
- Generic core crate for custom use-cases (such as [`bevy_mod_actuate`](https://github.com/actuate-rs/bevy_mod_actuate))

```rust
use actuate::prelude::*;

#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().start);

        Window::new((
            Text::new(format!("High five count: {}", *count))
                .font(GenericFamily::Cursive)
                .font_size(60.),
            Text::new("Up high")
                .on_click(move || Mut::update(count, |x| *x += 1))
                .background_color(Color::BLUE),
            Text::new("Down low")
                .on_click(move || Mut::update(count, |x| *x -= 1))
                .background_color(Color::RED),
            if *count == 0 {
                Some(Text::new("Gimme five!"))
            } else {
                None
            },
        ))
        .font_size(40.)
    }
}

fn main() {
    actuate::run(Counter { start: 0 })
}
```

## Borrowing
Composables can borrow from their ancestors, as well as state.
```rs
use actuate::prelude::*;

#[derive(Data)]
struct User<'a> {
    // `actuate::Cow` allows for either a borrowed or owned value.
    name: Cow<'a, String>,
}

impl Compose for User<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        // Get a mapped reference to the user's `name` field.
        let name = Ref::map(cx.me(), |me| &me.name);

        Text::new(name)
    }
}

#[derive(Data)]
struct App {
    name: String
}

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        // Get a mapped reference to the app's `name` field.
        let name = Ref::map(cx.me(), |me| &me.name).into();

        User { name }
    }
}

actuate::run(App { name: String::from("Matt") })
```

## Installation
To add this crate to your project:
```
cargo add actuate --features full
```

## Inspiration
This crate is inspired by [Xilem](https://github.com/linebender/xilem) and uses a similar approach to type-safe reactivity. The main difference with this crate is the concept of scopes, components store their state in their own scope and updates to that scope re-render the component.

State management is inspired by React and [Dioxus](https://github.com/DioxusLabs/dioxus).

Previous implementations were in [Concoct](https://github.com/concoct-rs/concoct) but were never very compatible with lifetimes.
