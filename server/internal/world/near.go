package world

import (
	"math"

	pb "github.com/sulram/planet/server/internal/protocol"
)

// The body's measures, as `topology` and `worldgen` hold them. Mirrored here
// and pinned by near_test.go to values the Rust side prints, so the two
// cannot drift unnoticed.
const (
	// Blocks per sector side, as a power of two. Every world is measured at
	// the reference size here; chat's world half measures by the recipe's own
	// (DECISIONS 99, ROADMAP.md).
	sectorBits = 16
	// Metres per block edge at a sector centre.
	blockM = 0.5
	// The moon's datum radius. Its stances ride the planet's sector grid.
	moonRadiusM = 8000.0
)

// The integer frame `[n, u, v]` of each sector, `topology::sector::FRAMES`.
var frames = [6][3][3]float64{
	{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}},
	{{-1, 0, 0}, {0, 0, 1}, {0, 1, 0}},
	{{0, 1, 0}, {0, 0, 1}, {1, 0, 0}},
	{{0, -1, 0}, {1, 0, 0}, {0, 0, 1}},
	{{0, 0, 1}, {1, 0, 0}, {0, 1, 0}},
	{{0, 0, -1}, {0, 1, 0}, {1, 0, 0}},
}

// direction is the unit vector from the body's centre through a surface
// point: the tangent warp of `topology::project`. False for a sector that
// does not exist.
func direction(sector uint32, u, v float64) ([3]float64, bool) {
	if sector >= uint32(len(frames)) {
		return [3]float64{}, false
	}
	half := float64(uint32(1) << (sectorBits - 1))
	warp := func(blocks float64) float64 { return math.Tan((blocks - half) / half * math.Pi / 4) }
	f := frames[sector]
	x, y := warp(u), warp(v)
	var d [3]float64
	var length float64
	for i := range d {
		d[i] = f[0][i] + f[1][i]*x + f[2][i]*y
		length += d[i] * d[i]
	}
	length = math.Sqrt(length)
	for i := range d {
		d[i] /= length
	}
	return d, true
}

// radiusBlocks is the datum radius of a body, in blocks. Four sector sides
// make one great circle of the planet; the moon has a radius of its own.
func radiusBlocks(body pb.Body) float64 {
	if body == pb.Body_BODY_MOON {
		return moonRadiusM / blockM
	}
	return float64(uint32(1)<<sectorBits) * 4 / (2 * math.Pi)
}

// apart is the distance between two stances in blocks: the great circle
// along the datum and the difference in height, taken together. False when
// they are on different bodies, or one of them is nowhere yet.
func apart(a, b *pb.Stance) (float64, bool) {
	if a == nil || b == nil || a.Body != b.Body {
		return 0, false
	}
	da, okA := direction(a.Sector, float64(a.U), float64(a.V))
	db, okB := direction(b.Sector, float64(b.U), float64(b.V))
	if !okA || !okB {
		return 0, false
	}
	dot := da[0]*db[0] + da[1]*db[1] + da[2]*db[2]
	angle := math.Acos(math.Max(-1, math.Min(1, dot)))
	along := angle * radiusBlocks(a.Body)
	up := (float64(a.HeightM) - float64(b.HeightM)) / blockM
	return math.Hypot(along, up), true
}
