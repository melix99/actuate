use crate::prelude::*;
use crate::{
    event_loop,
    ui::{Event, LayoutContext, WindowContext},
};
use parley::Rect;
use std::{
    cell::{Cell, RefCell},
    mem,
    num::NonZeroUsize,
    rc::Rc,
};
use taffy::{prelude::TaffyMaxContent, FlexDirection, NodeId, Size, Style, TaffyTree};
use vello::{
    self,
    kurbo::{Affine, Vec2},
    peniko::{Color, Fill},
    util::{RenderContext, RenderSurface},
    wgpu, AaConfig, RenderParams, Renderer, RendererOptions, Scene,
};
use wgpu::PresentMode;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    window::WindowAttributes,
};

struct State {
    renderer: Renderer,
    render_surface: RenderSurface<'static>,
}

/// Window composable.
#[derive(Data)]
#[must_use = "Composables do nothing unless composed with `actuate::run` or returned from other composables"]
pub struct Window<C> {
    /// Window attributes.
    pub attributes: WindowAttributes,
    /// Composable content.
    pub content: C,
    /// Background color.
    pub background_color: Color,
}

impl<C> Window<C> {
    /// Create a new window from its content.
    pub fn new(content: C) -> Self {
        Self {
            attributes: WindowAttributes::default(),
            content,
            background_color: Color::WHITE,
        }
    }
}

impl<C: Compose> Compose for Window<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let mut root_key_cell = None;
        let window_cx = use_provider(&cx, || {
            let mut taffy = TaffyTree::new();
            let root_key = taffy
                .new_leaf(Style {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                })
                .unwrap();
            root_key_cell = Some(root_key);

            let mut scene = Scene::new();
            scene.fill(
                Fill::NonZero,
                Affine::default(),
                Color::BLACK,
                None,
                &Rect::new(0., 0., 500., 500.),
            );

            WindowContext {
                scene: RefCell::new(scene),
                taffy: RefCell::new(taffy),
                is_changed: Cell::new(false),
                is_layout_changed: Cell::new(false),
                canvas_update_fns: RefCell::default(),
                listeners: Rc::default(),
                base_color: Cell::new(Color::WHITE),
            }
        });

        let layout_cx = use_provider(&cx, || LayoutContext {
            parent_id: root_key_cell.unwrap(),
        });

        let render_cx = use_ref(&cx, || RefCell::new(RenderContext::new()));

        let cursor_pos = use_ref(&cx, RefCell::default);
        let target = use_ref(&cx, || Cell::new(None));

        let state = use_ref(&cx, || RefCell::new(None));

        let is_first = use_ref(&cx, || Cell::new(true));

        event_loop::Window::new(
            WindowAttributes::default(),
            move |window, event| {
                if is_first.get() {
                    window_cx.scene.borrow_mut().fill(
                        Fill::NonZero,
                        Affine::default(),
                        window_cx.base_color.get(),
                        None,
                        &Rect::new(
                            0.,
                            0.,
                            window.inner_size().width as _,
                            window.inner_size().height as _,
                        ),
                    );
                    is_first.set(false);
                }

                match event {
                    WinitEvent::Resumed => {
                        let surface: RenderSurface<'_> =
                            pollster::block_on(render_cx.borrow_mut().create_surface(
                                window,
                                window.inner_size().width,
                                window.inner_size().height,
                                PresentMode::AutoVsync,
                            ))
                            .unwrap();

                        let renderer = Renderer::new(
                            &render_cx.borrow().devices[surface.dev_id].device,
                            RendererOptions {
                                surface_format: Some(surface.format),
                                use_cpu: false,
                                antialiasing_support: vello::AaSupport::all(),
                                num_init_threads: NonZeroUsize::new(1),
                            },
                        )
                        .unwrap();

                        // Safety: render_surface is valid for the lifetime of the window.
                        let render_surface: RenderSurface<'static> =
                            unsafe { mem::transmute(surface) };
                        *state.borrow_mut() = Some(State {
                            render_surface,
                            renderer,
                        })
                    }
                    WinitEvent::WindowEvent { event, .. } => match event {
                        WindowEvent::CursorMoved { position, .. } => {
                            *cursor_pos.borrow_mut() = Vec2::new(position.x, position.y);

                            let pos = *cursor_pos.borrow();
                            let taffy = window_cx.taffy.borrow();

                            if let Some(id) = hit_test(&taffy, pos, layout_cx) {
                                if let Some(last_id) = target.replace(Some(id)) {
                                    if last_id != id {
                                        if let Some(listeners) =
                                            window_cx.listeners.borrow().get(&last_id)
                                        {
                                            for f in listeners {
                                                f(Event::MouseOut)
                                            }
                                        }

                                        if let Some(listeners) =
                                            window_cx.listeners.borrow().get(&id)
                                        {
                                            for f in listeners {
                                                f(Event::MouseIn)
                                            }
                                        }
                                    }
                                } else if let Some(listeners) =
                                    window_cx.listeners.borrow().get(&id)
                                {
                                    for f in listeners {
                                        f(Event::MouseIn)
                                    }
                                }

                                if let Some(listeners) = window_cx.listeners.borrow().get(&id) {
                                    for f in listeners {
                                        f(Event::MouseMove { pos })
                                    }
                                }
                            }
                        }
                        WindowEvent::MouseInput { button, state, .. } => {
                            let pos = *cursor_pos.borrow();
                            let taffy = window_cx.taffy.borrow();

                            let mut keys = vec![(Vec2::default(), layout_cx.parent_id)];

                            let mut target = None;

                            while let Some((parent_pos, key)) = keys.pop() {
                                let layout = taffy.layout(key).unwrap();
                                if pos.x >= parent_pos.x + layout.location.x as f64
                                    && pos.y >= parent_pos.y + layout.location.y as f64
                                    && pos.x
                                        <= parent_pos.x
                                            + layout.location.x as f64
                                            + layout.size.width as f64
                                    && pos.y
                                        <= parent_pos.y
                                            + layout.location.y as f64
                                            + layout.size.height as f64
                                {
                                    target = Some(key);

                                    keys.extend(taffy.children(key).unwrap().into_iter().map(
                                        |key| {
                                            (
                                                parent_pos
                                                    + Vec2::new(
                                                        layout.location.x as _,
                                                        layout.location.y as _,
                                                    ),
                                                key,
                                            )
                                        },
                                    ));
                                }
                            }

                            if let Some(key) = target {
                                if let Some(listeners) = window_cx.listeners.borrow().get(&key) {
                                    for f in listeners {
                                        f(Event::MouseInput {
                                            button: *button,
                                            state: *state,
                                            pos: *cursor_pos.borrow(),
                                        })
                                    }
                                }
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            #[cfg(feature = "tracing")]
                            tracing::trace!("Redraw");

                            let Some(state) = &mut *state.borrow_mut() else {
                                return;
                            };

                            let texture =
                                state.render_surface.surface.get_current_texture().unwrap();
                            let mut scene = window_cx.scene.borrow_mut();
                            let device_handle =
                                &render_cx.borrow().devices[state.render_surface.dev_id];

                            state
                                .renderer
                                .render_to_surface(
                                    &device_handle.device,
                                    &device_handle.queue,
                                    &scene,
                                    &texture,
                                    &RenderParams {
                                        base_color: Color::BLACK,
                                        width: window.inner_size().width,
                                        height: window.inner_size().height,
                                        antialiasing_method: AaConfig::Msaa16,
                                    },
                                )
                                .unwrap();

                            texture.present();
                            device_handle.device.poll(wgpu::Maintain::Poll);

                            scene.reset();
                            scene.fill(
                                Fill::NonZero,
                                Affine::default(),
                                window_cx.base_color.get(),
                                None,
                                &Rect::new(
                                    0.,
                                    0.,
                                    window.inner_size().width as _,
                                    window.inner_size().height as _,
                                ),
                            );
                        }
                        _ => {}
                    },
                    _ => {}
                }

                if window_cx.is_changed.take() {
                    window.request_redraw();

                    for f in window_cx.canvas_update_fns.borrow().values() {
                        f()
                    }
                }

                if window_cx.is_layout_changed.take() {
                    window_cx
                        .taffy
                        .borrow_mut()
                        .compute_layout(layout_cx.parent_id, Size::MAX_CONTENT)
                        .unwrap();
                }
            },
            Ref::map(cx.me(), |me| &me.content),
        )
    }
}

fn hit_test(taffy: &TaffyTree, pos: Vec2, layout_cx: &LayoutContext) -> Option<NodeId> {
    let mut keys = vec![(Vec2::default(), layout_cx.parent_id)];

    let mut target = None;

    while let Some((parent_pos, key)) = keys.pop() {
        let layout = taffy.layout(key).unwrap();
        if pos.x >= parent_pos.x + layout.location.x as f64
            && pos.y >= parent_pos.y + layout.location.y as f64
            && pos.x <= parent_pos.x + layout.location.x as f64 + layout.size.width as f64
            && pos.y <= parent_pos.y + layout.location.y as f64 + layout.size.height as f64
        {
            target = Some(key);

            keys.extend(taffy.children(key).unwrap().into_iter().map(|key| {
                (
                    parent_pos + Vec2::new(layout.location.x as _, layout.location.y as _),
                    key,
                )
            }));
        }
    }

    target
}
