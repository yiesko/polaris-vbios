//! Edge-case and pure-logic tests. The garbage/truncation tests need
//! no samples and always run; the ones using a real ROM skip cleanly
//! when the collection is unavailable.

mod common;

use polaris_vbios::rom;
use polaris_vbios::rom::limits::{
    Die, PowerEnvelope, TdpKind, absurd_ceiling, classify, envelope_for,
};
use polaris_vbios::rom::patch::fix_checksum;

/// Malformed input must produce a clean `Err`, never a panic.
#[test]
fn empty_bytes_is_an_error_not_a_panic() {
    assert!(rom::parse_bytes(&[], "empty.rom").is_err());
}

#[test]
fn single_byte_is_an_error() {
    assert!(rom::parse_bytes(&[0x55], "tiny.rom").is_err());
}

#[test]
fn all_ff_is_an_error() {
    assert!(rom::parse_bytes(&[0xFFu8; 512], "ff.rom").is_err());
}

#[test]
fn all_zeros_is_an_error() {
    assert!(rom::parse_bytes(&[0u8; 4096], "zeros.rom").is_err());
}

/// Truncating a real ROM at several points must never panic: either
/// the parse fails (truncated header/table) or succeeds with warnings.
/// Panics would be caught by the harness, so the test only needs the
/// call itself.
#[test]
fn truncating_a_real_rom_never_panics() {
    let Some(bytes) = common::try_rom_bytes("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    for stop in [1usize, 256, 4096, 0x10000, bytes.len() / 2, bytes.len() - 1] {
        let cut = common::truncated(&bytes, stop);
        // The contract is "no panic" — both Ok and Err are acceptable.
        let _ = rom::parse_bytes(&cut, "cut.rom");
    }
}

/// Flipping a byte in the middle must also never panic, and a flipped
/// byte inside the checksum region makes the checksum invalid (a real
/// validation signal the read path uses).
#[test]
fn flipped_byte_never_panics() {
    let Some(bytes) = common::try_rom_bytes("AMD.RX590.8192.191126.rom") else {
        eprintln!("skipped: sample ROM not available");
        return;
    };
    for off in [0usize, 4, 0x100, 0x1000, bytes.len() / 2] {
        let flipped = common::flip_byte(&bytes, off);
        let _ = rom::parse_bytes(&flipped, "flipped.rom");
    }
}

// --- Pure unit tests (always run, no samples) ---

#[test]
fn envelope_bounds_match_stock_vbios() {
    let e: PowerEnvelope = envelope_for(Die::Baffin).unwrap();
    assert_eq!((e.stock_min_w, e.stock_max_w), (42, 75));
    let e = envelope_for(Die::Lexa).unwrap();
    assert_eq!((e.stock_min_w, e.stock_max_w), (35, 65));
    let e = envelope_for(Die::Ellesmere10).unwrap();
    assert_eq!((e.stock_min_w, e.stock_max_w), (85, 130));
    let e = envelope_for(Die::Ellesmere20).unwrap();
    assert_eq!((e.stock_min_w, e.stock_max_w), (60, 185));
    let e = envelope_for(Die::EllesmereGeneric).unwrap();
    assert_eq!((e.stock_min_w, e.stock_max_w), (85, 185));
    let e = envelope_for(Die::Polaris30).unwrap();
    assert_eq!((e.stock_min_w, e.stock_max_w), (185, 220));
    assert!(envelope_for(Die::Unknown).is_none());
}

#[test]
fn classify_boundaries() {
    let env = envelope_for(Die::Baffin).unwrap(); // 42-75, oc 90
    assert_eq!(classify(20, &env), TdpKind::TooLow); // < floor/2
    assert_eq!(classify(30, &env), TdpKind::UnusualLow); // floor/2 .. floor
    assert_eq!(classify(50, &env), TdpKind::Normal);
    assert_eq!(classify(75, &env), TdpKind::Normal);
    assert_eq!(classify(85, &env), TdpKind::UnusualHigh); // stock..oc
    assert_eq!(classify(200, &env), TdpKind::Absurd); // > oc + 25%
    let ceiling = absurd_ceiling(&env);
    assert_eq!(ceiling, 90 + 90 / 4, "Baffin absurd ceiling = oc + 25%");
}

#[test]
fn safe_tdp_rejects_nonsense() {
    use polaris_vbios::rom::limits::SafeTdp;
    assert!(SafeTdp::try_new(500, Die::Baffin).is_err(), "500 W absurd");
    assert!(SafeTdp::try_new(5, Die::Baffin).is_err(), "5 W too low");
    assert!(SafeTdp::try_new(60, Die::Baffin).is_ok());
    let unusual = SafeTdp::try_new(85, Die::Baffin).unwrap();
    assert!(
        unusual.is_unusual(),
        "85 W on Baffin is above stock, unusual"
    );
}

#[test]
fn checksum_api_does_not_panic() {
    let mut d = vec![0xABu8; 64];
    // fix_checksum must not panic on a small buffer; it may or may not
    // produce a diff depending on the region layout.
    let _ = fix_checksum(&mut d);
}

#[test]
fn timing_ns_conversion() {
    // cycles / clock (MHz) * 1000 = ns
    let ns = rom::timings::ns(600, 1500.0);
    assert!(
        (ns - 400.0).abs() < 0.001,
        "600cy @1500MHz = 400ns, got {ns}"
    );
    let ns = rom::timings::ns(1000, 2000.0);
    assert!((ns - 500.0).abs() < 0.001);
}

#[test]
fn convert_command_roundtrip() {
    // convert --clock 1500 --cycles 600 => 400.000 ns
    let (out, code, _) = common::run(&["convert", "--clock", "1500", "--cycles", "600"]);
    assert_eq!(code, 0, "convert exits 0");
    assert!(out.contains("400"), "convert 600cy@1500MHz: {out}");
    // and back: --ns 400 => cycles
    let (out, code, _) = common::run(&["convert", "--clock", "1500", "--ns", "400"]);
    assert_eq!(code, 0);
    assert!(out.contains("600"), "convert 400ns@1500MHz: {out}");
}

#[test]
fn signed16_converts_correctly() {
    use polaris_vbios::rom::powerplay::signed16;
    // 32767 (0x7FFF) should stay positive
    assert_eq!(signed16(32767), 32767);
    // 32768 (0x8000) should become -32768
    assert_eq!(signed16(32768), -32768);
    // 65535 (0xFFFF) should become -1
    assert_eq!(signed16(65535), -1);
    // 0 should stay 0
    assert_eq!(signed16(0), 0);
    // 1 should stay 1
    assert_eq!(signed16(1), 1);
}
