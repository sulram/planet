//! Places as people say them: a short code for a spot in address space.
//!
//! The code is the address, not a name beside it. It is read off `(sector, u,
//! v)` by interleaving their bits and spelling the result in base32, so:
//!
//! - a **prefix is a box**. Two places whose codes start alike are near each
//!   other, and cutting characters off widens the box around the same spot.
//! - **length is precision**. Four characters name about sixteen metres of the
//!   largest body, seven name one block.
//! - it is **integer all the way down**, so it never disagrees with the save
//!   format, and it survives being said out loud, written on paper by someone
//!   else and pasted into a URL.
//!
//! A height rides along as `@h`, blocks from the datum, and it is left out
//! when the ground decides it. On the surface that is nearly always, which is
//! why the short form is the common one; inside a tower every floor is the
//! same column, and there the height is the whole of what is being said.
//!
//! A body is not in here. `topology` knows one grid at a time and a world may
//! have a planet and a moon, so whoever holds more than one body says which.

use crate::address::Column;
use crate::grid::{Grid, MAX_BITS};
use crate::sector::Sector;
use crate::vec3::{self, Vec3};

/// Crockford's base32: no `I`, `L`, `O` or `U`, because those are the ones
/// misread on paper and misheard out loud.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Bits one character carries.
const BITS_PER_CHAR: u32 = 5;

/// What separates a height from the code. Reads as "at", and it is not in the
/// alphabet, so it can never be mistaken for one more character of precision.
pub const HEIGHT_MARK: char = '@';

/// Characters that name a single block of the largest body there is. `u` and
/// `v` are [`MAX_BITS`] each, so the interleaved stream is twice that, and the
/// last character is padded.
pub const CODE_MAX: usize = 7;

/// Why a code could not be read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlaceError {
    /// No sector digit, or one outside `0..6`.
    Sector,
    /// A character that is not in the alphabet.
    Character,
    /// No characters after the sector, or more than [`CODE_MAX`].
    Length,
    /// Something after `@` that is not a height in blocks.
    Height,
}

/// A place and how precisely it was named.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Place {
    /// The lowest corner of the box the code names, on the grid it was read
    /// for. A code shorter than [`CODE_MAX`] names a box, not a column.
    pub column: Column,
    /// Blocks from the datum sphere, when the code said. `None` means the
    /// ground decides, which is what standing on it means.
    pub h: Option<i16>,
    /// Characters given, `1..=CODE_MAX`.
    pub chars: usize,
}

impl Place {
    /// How wide the box this code names is, in columns of `grid`: `[u, v]`.
    ///
    /// Dropping a character doubles one side, because a character is five bits
    /// and the stream alternates: an odd number of them leaves `u` one bit
    /// ahead, so the box is twice as long as it is wide. Geohash has the same
    /// shape for the same reason, and pretending otherwise would make the code
    /// lie about what it names.
    pub fn span(self, grid: Grid) -> [u32; 2] {
        let (u_bits, v_bits) = split(self.chars);
        [u_bits, v_bits].map(|bits| 1u32 << (grid.bits() - bits.min(grid.bits())))
    }
}

/// Bits of `u` and of `v` that `chars` characters carry, before the grid caps
/// them. The stream alternates starting with `u`, so `u` is never behind.
fn split(chars: usize) -> (u32, u32) {
    let bits = (chars as u32) * BITS_PER_CHAR;
    (bits.div_ceil(2), bits / 2)
}

/// The code for a place, to `chars` characters: `"4-K7M42Q"`, or
/// `"4-K7M42Q@128"` when a height is given.
///
/// The column is read on `grid` and written against the largest body, so the
/// same code names the same fraction of a sector whatever size a world is.
/// Pass `None` for `h` when the ground decides the height, which is what
/// standing on it means and what nearly every place outdoors is.
pub fn code(grid: Grid, column: Column, h: Option<i16>, chars: usize) -> String {
    let chars = chars.clamp(1, CODE_MAX);
    let shift = MAX_BITS - grid.bits();
    let stream = interleave(u32::from(column.u) << shift, u32::from(column.v) << shift);
    // The stream is 2 * MAX_BITS bits and characters want a multiple of five,
    // so it is left aligned in what the characters hold: the last one is
    // padded from the right rather than any of the address being dropped.
    let width = 2 * MAX_BITS;
    let total = (chars as u32) * BITS_PER_CHAR;
    let aligned = if total >= width {
        stream << (total - width)
    } else {
        stream >> (width - total)
    };
    let mut out = String::with_capacity(chars + 2);
    out.push(ALPHABET[column.sector.index()] as char);
    out.push('-');
    for i in 0..chars as u32 {
        let high = total - (i + 1) * BITS_PER_CHAR;
        out.push(ALPHABET[((aligned >> high) & 0x1f) as usize] as char);
    }
    if let Some(h) = h {
        out.push(HEIGHT_MARK);
        out.push_str(itoa(h).as_str());
    }
    out
}

/// `i16` as text without pulling in a formatter, so this stays cheap on the
/// hot path a HUD sits on.
fn itoa(value: i16) -> String {
    let mut out = String::with_capacity(6);
    if value < 0 {
        out.push('-');
    }
    let mut digits = [0u8; 5];
    let mut n = value.unsigned_abs();
    let mut at = 0;
    loop {
        digits[at] = b'0' + (n % 10) as u8;
        n /= 10;
        at += 1;
        if n == 0 {
            break;
        }
    }
    for i in (0..at).rev() {
        out.push(digits[i] as char);
    }
    out
}

/// Reads a code back. Case is ignored and the dash is optional, because
/// neither survives being copied by hand.
pub fn place(grid: Grid, text: &str) -> Result<Place, PlaceError> {
    // The height is split off first: it is decimal and signed, and none of
    // that survives being folded into the alphabet.
    let (text, h) = match text.split_once(HEIGHT_MARK) {
        None => (text, None),
        Some((code, height)) => (
            code,
            Some(
                height
                    .trim()
                    .parse::<i16>()
                    .map_err(|_| PlaceError::Height)?,
            ),
        ),
    };
    let mut chars = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase());
    let first = chars.next().ok_or(PlaceError::Sector)?;
    let sector =
        Sector::new(value_of(first).ok_or(PlaceError::Sector)? as u8).ok_or(PlaceError::Sector)?;

    let mut stream = 0u64;
    let mut given = 0usize;
    for c in chars {
        let value = value_of(c).ok_or(PlaceError::Character)?;
        if given == CODE_MAX {
            return Err(PlaceError::Length);
        }
        stream = (stream << BITS_PER_CHAR) | u64::from(value);
        given += 1;
    }
    if given == 0 {
        return Err(PlaceError::Length);
    }

    // Left align what was given in the full stream, so a short code is the top
    // of the box and the bits nobody gave are zero.
    let width = 2 * MAX_BITS;
    let bits = (given as u32) * BITS_PER_CHAR;
    let stream = if bits >= width {
        stream >> (bits - width)
    } else {
        stream << (width - bits)
    };
    let (u, v) = deinterleave(stream);
    let shift = MAX_BITS - grid.bits();
    Ok(Place {
        column: Column::new(sector, (u >> shift) as u16, (v >> shift) as u16),
        h,
        chars: given,
    })
}

/// How close to a pole a bearing stops meaning anything: within this much of
/// the axis, "north" is shorter than the rounding and every way is south.
const POLE_EPSILON: f64 = 1e-6;

/// Degrees clockwise from north, `0..360`, for a body whose local up is `up`
/// looking along `facing`. Neither needs to be unit.
///
/// North is the `+Y` pole, which is the middle of sector 2. That is not a
/// choice made here: the sun turns about `+Y`, so it is the axis that gives a
/// world its time zones, and a compass has to agree with the sky.
///
/// `None` at a pole, where a bearing is not a thing that exists. The caller
/// says so rather than being handed a number that spins.
///
/// The letters are not here. N, S, E and W are English, and a user visible
/// string belongs to whoever has the locale, so this returns the angle.
pub fn bearing_deg(up: Vec3, facing: Vec3) -> Option<f64> {
    const NORTH: Vec3 = [0.0, 1.0, 0.0];
    let up = vec3::normalize(up);
    // North on the ground: the pole, with the part of it that points at the
    // sky taken out.
    let north = vec3::sub(NORTH, vec3::scale(up, vec3::dot(up, NORTH)));
    if vec3::length(north) < POLE_EPSILON {
        return None;
    }
    let north = vec3::normalize(north);
    // `(east, north, up)` right handed, so east is where the ground turns to
    // when the sun rises.
    let east = vec3::cross(north, up);
    let flat = vec3::sub(facing, vec3::scale(up, vec3::dot(up, facing)));
    if vec3::length(flat) < POLE_EPSILON {
        return None;
    }
    let degrees = libm::atan2(vec3::dot(flat, east), vec3::dot(flat, north)).to_degrees();
    Some(if degrees < 0.0 {
        degrees + 360.0
    } else {
        degrees
    })
}

fn value_of(c: char) -> Option<u32> {
    ALPHABET
        .iter()
        .position(|&a| a == c as u8)
        .map(|at| at as u32)
}

/// `u` and `v` woven together, most significant first, `u` leading. The
/// alternation is what makes a prefix a box: every pair of bits halves both.
fn interleave(u: u32, v: u32) -> u64 {
    let mut out = 0u64;
    for i in (0..MAX_BITS).rev() {
        out = (out << 1) | u64::from((u >> i) & 1);
        out = (out << 1) | u64::from((v >> i) & 1);
    }
    out
}

fn deinterleave(stream: u64) -> (u32, u32) {
    let (mut u, mut v) = (0u32, 0u32);
    for i in (0..MAX_BITS).rev() {
        let at = 2 * i;
        v = (v << 1) | ((stream >> at) & 1) as u32;
        u = (u << 1) | ((stream >> (at + 1)) & 1) as u32;
    }
    (u, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(bits: u32) -> Grid {
        Grid::new(bits).expect("a grid")
    }

    #[test]
    fn a_full_code_round_trips_on_every_grid() {
        for bits in [4, 8, 10, 16] {
            let g = grid(bits);
            let last = g.max_coord();
            for column in [
                Column::new(Sector::ALL[0], 0, 0),
                Column::new(Sector::ALL[3], last, 0),
                Column::new(Sector::ALL[5], last, last),
                Column::new(Sector::ALL[2], last / 3, last / 7),
            ] {
                let text = code(g, column, None, CODE_MAX);
                let back = place(g, &text).expect("reads back");
                assert_eq!(back.column, column, "{text} on 2^{bits}");
            }
        }
    }

    #[test]
    fn a_shorter_code_is_the_box_around_the_longer_one() {
        let g = grid(16);
        let column = Column::new(Sector::ALL[4], 40_000, 9_001);
        let full = code(g, column, None, CODE_MAX);
        for chars in 1..CODE_MAX {
            let short = code(g, column, None, chars);
            assert!(
                full.starts_with(&short),
                "{short} should be a prefix of {full}"
            );
            let box_ = place(g, &short).expect("reads back");
            let [su, sv] = box_.span(g).map(i64::from);
            let du = i64::from(column.u) - i64::from(box_.column.u);
            let dv = i64::from(column.v) - i64::from(box_.column.v);
            assert!(
                (0..su).contains(&du) && (0..sv).contains(&dv),
                "{short} spans {su}x{sv} but misses {column:?} by {du},{dv}"
            );
        }
    }

    #[test]
    fn a_code_survives_being_copied_by_hand() {
        let g = grid(16);
        let column = Column::new(Sector::ALL[1], 1234, 43210);
        let text = code(g, column, None, CODE_MAX);
        for variant in [
            text.to_lowercase(),
            text.replace('-', ""),
            format!(" {text} "),
        ] {
            assert_eq!(place(g, &variant).expect("reads back").column, column);
        }
    }

    /// Half a degree: a compass rose has sixteen points and each is 22.5
    /// degrees wide, so this is finer than anything a person reads off one.
    fn about(got: Option<f64>, want: f64) {
        let got = got.expect("a bearing away from the poles");
        assert!((got - want).abs() < 0.5, "bearing {got}, wanted {want}");
    }

    #[test]
    fn a_height_rides_along_and_is_left_out_when_the_ground_decides() {
        let g = grid(16);
        let column = Column::new(Sector::ALL[2], 700, 800);
        assert_eq!(place(g, &code(g, column, None, CODE_MAX)).unwrap().h, None);
        for h in [0i16, 1, -1, 128, -256, i16::MAX, i16::MIN] {
            let text = code(g, column, Some(h), CODE_MAX);
            let back = place(g, &text).expect("reads back");
            assert_eq!(back.column, column, "{text}");
            assert_eq!(back.h, Some(h), "{text}");
        }
        // A tower is one column and many floors, which is the whole point.
        let ground = code(g, column, None, CODE_MAX);
        assert!(code(g, column, Some(40), CODE_MAX).starts_with(&ground));
        assert_eq!(place(g, "4-K7M42Q@up"), Err(PlaceError::Height));
    }

    #[test]
    fn a_compass_agrees_with_the_pole_and_turns_the_right_way() {
        // Standing on the equator at +X, where up is +X and the pole is +Y.
        let up = [1.0, 0.0, 0.0];
        about(bearing_deg(up, [0.0, 1.0, 0.0]), 0.0);
        about(bearing_deg(up, [0.0, -1.0, 0.0]), 180.0);
        // East is where the ground turns to: (east, north, up) right handed.
        about(bearing_deg(up, [0.0, 0.0, -1.0]), 90.0);
        about(bearing_deg(up, [0.0, 0.0, 1.0]), 270.0);
        // Halfway between north and east reads as the corner of the rose.
        about(bearing_deg(up, [0.0, 1.0, -1.0]), 45.0);
    }

    #[test]
    fn at_a_pole_there_is_no_bearing_rather_than_a_wrong_one() {
        assert_eq!(bearing_deg([0.0, 1.0, 0.0], [1.0, 0.0, 0.0]), None);
        assert_eq!(bearing_deg([0.0, -1.0, 0.0], [1.0, 0.0, 0.0]), None);
        // Looking straight up is not a direction on the ground either.
        assert_eq!(bearing_deg([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]), None);
    }

    #[test]
    fn nonsense_is_refused_rather_than_guessed() {
        let g = grid(16);
        assert_eq!(place(g, ""), Err(PlaceError::Sector));
        assert_eq!(place(g, "9-ABC"), Err(PlaceError::Sector));
        assert_eq!(place(g, "4-"), Err(PlaceError::Length));
        // I, L, O and U are not in the alphabet on purpose.
        assert_eq!(place(g, "4-ABCIDEF"), Err(PlaceError::Character));
        assert_eq!(place(g, "4-ABCDEFGH"), Err(PlaceError::Length));
    }
}
