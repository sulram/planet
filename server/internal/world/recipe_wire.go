package world

import pb "github.com/sulram/planet/server/internal/protocol"

// Wire is the recipe as Welcome carries it, so a client can check it stands
// in the world the page said, or build the world from it alone.
func (r Recipe) Wire() *pb.Recipe {
	return &pb.Recipe{
		Seed:             r.Seed.String(),
		GeneratorVersion: uint32(r.GeneratorVersion),
		ParamsJson:       string(r.Params),
	}
}
