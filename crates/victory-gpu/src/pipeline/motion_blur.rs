use std::sync::LazyLock;
use victory_shader::{Container, HwStage, NativeProgram, ShaderError, emit, io, resource, spill, structure};

// The exact PS_MotionBlurLowRes found in this title's half-resolution pass.
const LOW_RES: &str = "c19a0b6c544d5e78c319a4086f21c62a97bfd428320a913918ce9a77e7cc6628";
static ENABLED: LazyLock<bool> = LazyLock::new(|| {
    std::env::var_os("VICTORY_MOTION_BLUR_OFF").is_some_and(|value| value == "1")
});

/// Included in the offline pipeline completion stamp when this override is active.
pub(crate) fn revision() -> Option<&'static [u8]> {
    (*ENABLED).then_some(b"motion-blur-low-res-neutral-v1")
}

pub(super) fn neutral_pass(
    bytes: &[u8],
    container: &Container<'_>,
    programs: &[NativeProgram],
) -> Option<Result<emit::HostShader, ShaderError>> {
    if revision().is_none() || bytes.len() != 2164 || blake3::hash(bytes).to_hex().as_str() != LOW_RES {
        return None;
    }
    let mut ps = programs.iter().find(|p| p.stage == HwStage::Ps)?.clone();
    Some((|| {
        // This shader initializes its RGBA output to zero, accumulates motion
        // samples only for velocities of at least two pixels, and always writes
        // the render target. The following composite uses base*(1-a)+rgb.
        // Select the author's zero-velocity path; keep the input structure,
        // output signature, export layout, and draw intact.
        let speed = ps.code.get_mut(0x94..0x98).ok_or(ShaderError::Malformed("missing motion magnitude"))?;
        if speed != 0x0e1a16f0u32.to_le_bytes() { // v_mul_legacy_f32 v13, 0.5, v11
            return Err(ShaderError::Malformed("unexpected motion magnitude instruction"));
        }
        speed.copy_from_slice(&0x7e1a0280u32.to_le_bytes()); // v_mov_b32 v13, 0
        let ir = spill::build(&ps)?;
        let control = structure::structure(&ir)?;
        let resources = resource::track(&ir, &ps.layout, ps.stage, Some(&control))?;
        let signature = io::raise(&ir, &ps)?;
        let shader = emit::emit(container, &ps, &ir, &control, &signature, &resources)?;
        tracing::info!("motion blur: low-resolution pass uses its native zero-velocity path");
        Ok(shader)
    })())
}
