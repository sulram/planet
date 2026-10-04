package world

import (
	"math"
	"testing"

	pb "github.com/sulram/planet/server/internal/protocol"
)

// The same points, the same numbers, on the Rust side:
// crates/topology/tests/goldens.rs. Change one, change both.
func TestDirectionMirrorsTopology(t *testing.T) {
	cases := []struct {
		sector uint32
		u, v   float64
		want   [3]float64
	}{
		{0, 32768, 32768, [3]float64{1, 0, 0}},
		{2, 40000.25, 9001.5, [3]float64{-0.533564823189042, 0.833083947757445, 0.145875684896822}},
		{5, 0, 65536, [3]float64{0.577350269189626, -0.577350269189626, -0.577350269189626}},
		{3, 65535.5, 12, [3]float64{0.577451717419852, -0.577465558208231, -0.577133470812531}},
		{4, 1000, 64000, [3]float64{-0.572553412021728, 0.558002585774256, 0.600679369257445}},
	}
	for _, c := range cases {
		got, ok := direction(c.sector, c.u, c.v)
		if !ok {
			t.Fatalf("sector %d exists", c.sector)
		}
		for i := range got {
			if math.Abs(got[i]-c.want[i]) > 1e-12 {
				t.Fatalf("direction(%d, %v, %v) = %v, want %v", c.sector, c.u, c.v, got, c.want)
			}
		}
	}
	if _, ok := direction(6, 0, 0); ok {
		t.Fatal("there is no seventh sector")
	}
}

func TestApartIsBlocks(t *testing.T) {
	at := func(body pb.Body, sector uint32, u, v, height float32) *pb.Stance {
		return &pb.Stance{Body: body, Sector: sector, U: u, V: v, HeightM: height}
	}
	centre := at(pb.Body_BODY_PLANET, 2, 32768, 32768, 0)

	// A block is a block at a sector centre, along the ground and up.
	if d, _ := apart(centre, at(pb.Body_BODY_PLANET, 2, 32778, 32768, 0)); math.Abs(d-10) > 0.01 {
		t.Fatalf("ten blocks along u: %v", d)
	}
	if d, _ := apart(centre, at(pb.Body_BODY_PLANET, 2, 32768, 32768, 5)); math.Abs(d-10) > 1e-9 {
		t.Fatalf("five metres up is ten blocks: %v", d)
	}
	// Across a seam the distance is still the distance.
	// Sector 0 looks out along +X with v along +Z; sector 4 looks out along
	// +Z with u along +X. A block short of the seam on each side.
	edge := at(pb.Body_BODY_PLANET, 0, 32768, 65535, 0)
	over := at(pb.Body_BODY_PLANET, 4, 65535, 32768, 0)
	if d, _ := apart(edge, over); d > 3 {
		t.Fatalf("a step over the seam is a step: %v", d)
	}
	// The moon is its own body, and smaller.
	if _, same := apart(centre, at(pb.Body_BODY_MOON, 2, 32768, 32768, 0)); same {
		t.Fatal("the moon is not near the planet")
	}
	moon := at(pb.Body_BODY_MOON, 2, 32768, 32768, 0)
	if d, _ := apart(moon, at(pb.Body_BODY_MOON, 2, 32778, 32768, 0)); d > 5 {
		t.Fatalf("ten planet blocks are fewer on the moon: %v", d)
	}
	if _, same := apart(centre, nil); same {
		t.Fatal("nowhere is not near")
	}
	// The warp stretches a block a little away from the centre, so the edge
	// of a reach is asked about with a margin. A room measures it for a plugin.
	const reach = 64
	near := func(to *pb.Stance) bool { return room{}.Near(Who{Stance: centre}, Who{Stance: to}, reach) }
	if !near(at(pb.Body_BODY_PLANET, 2, 32768+reach-4, 32768, 0)) || near(at(pb.Body_BODY_PLANET, 2, 32768+reach+4, 32768, 0)) {
		t.Fatal("near ends at the reach asked about")
	}
}
