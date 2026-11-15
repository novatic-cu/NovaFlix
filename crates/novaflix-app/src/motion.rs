//! Muelles interrumpibles (design-system §3.8). Cada valor animado vive en la memoria de
//! `egui` bajo un `Id`; si cambia el objetivo a mitad de camino, continúa desde donde está.

use egui::{Context, Id};

#[derive(Clone, Copy)]
struct Spring {
    value: f32,
    velocity: f32,
}

/// Parámetros de un muelle: respuesta (s) y amortiguación (0..1+).
#[derive(Clone, Copy)]
pub struct Params {
    pub response: f32,
    pub damping: f32,
}

pub const SNAPPY: Params = Params {
    response: 0.30,
    damping: 0.85,
};
pub const SMOOTH: Params = Params {
    response: 0.45,
    damping: 0.90,
};

pub fn reduce_motion(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(Id::new("reduce-motion")))
        .unwrap_or(false)
}

pub fn set_reduce_motion(ctx: &Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(Id::new("reduce-motion"), on));
}

/// Avanza el muelle hacia `target` y devuelve el valor actual. Pide repintado mientras se mueve.
pub fn animate(ctx: &Context, id: Id, target: f32, params: Params) -> f32 {
    if reduce_motion(ctx) {
        return target;
    }
    let key = id.with("spring");
    let mut s = ctx.data(|d| d.get_temp::<Spring>(key)).unwrap_or(Spring {
        value: target,
        velocity: 0.0,
    });

    let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.05);
    // Muelle amortiguado semi-implícito: ω = 2π / respuesta, ζ = amortiguación.
    let omega = std::f32::consts::TAU / params.response;
    let accel = omega * omega * (target - s.value) - 2.0 * params.damping * omega * s.velocity;
    s.velocity += accel * dt;
    s.value += s.velocity * dt;

    let settled = (target - s.value).abs() < 0.001 && s.velocity.abs() < 0.01;
    if settled {
        s = Spring {
            value: target,
            velocity: 0.0,
        };
    } else {
        ctx.request_repaint();
    }
    ctx.data_mut(|d| d.insert_temp(key, s));
    s.value
}
