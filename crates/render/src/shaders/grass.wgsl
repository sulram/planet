// Three curved blades per tuft, roots fixed on the drawn terrain.
struct Patch { offset: vec4<f32>, anchor: vec4<f32> }
@group(1) @binding(0) var<uniform> placement: Patch;

struct Instance {
    @location(0) root_height: vec4<f32>,
    @location(1) up_reach: vec4<f32>,
    @location(2) color_angle: vec4<f32>,
}
struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) tip: f32,
    @location(4) far: f32,
}

@vertex
fn vs(in: Instance, @builtin(vertex_index) index: u32) -> Varying {
    let shape = array<vec2<f32>,9>(
        vec2<f32>(-1.0,0.0), vec2<f32>(1.0,0.0), vec2<f32>(-0.65,0.55),
        vec2<f32>(1.0,0.0), vec2<f32>(0.65,0.55), vec2<f32>(-0.65,0.55),
        vec2<f32>(-0.65,0.55), vec2<f32>(0.65,0.55), vec2<f32>(0.0,1.0));
    let p = shape[index % 9u];
    let up = normalize(in.up_reach.xyz);
    let root = in.root_height.xyz + placement.offset.xyz;
    let helper = select(vec3<f32>(0.0,1.0,0.0), vec3<f32>(1.0,0.0,0.0), abs(up.y) > 0.9);
    let east = normalize(cross(helper,up));
    let north = cross(up,east);
    let angle = in.color_angle.w + f32(index/9u)*2.094395;
    let side = east*cos(angle)+north*sin(angle);
    let forward = cross(side,up);
    let distance = length(root);
    let fade = 1.0-smoothstep(in.up_reach.w*0.68,in.up_reach.w,distance);
    // Far tufts stand for the meadow between them: taller and wider, so the
    // field keeps its cover while the count falls.
    let far = smoothstep(35.0,400.0,distance);
    let height = in.root_height.w * fade * (1.0+far*1.4);
    let anchored = placement.anchor.xyz + in.root_height.xyz;
    // Integer periods keep wind continuous when the scene clock wraps at 3600s.
    let t = view.flags.y * 6.2831853;
    let wind = sin(dot(anchored,vec3<f32>(1.0,2.0,1.0)*(6.2831853/64.0))+t/6.0)*0.16
        + sin(t/2.0+in.color_angle.w)*0.035;
    let axis = view.interaction_end.xyz-view.interaction_start.xyz;
    let along = clamp(dot(root-view.interaction_start.xyz,axis)/max(dot(axis,axis),0.001),0.0,1.0);
    let away = root-(view.interaction_start.xyz+axis*along);
    let influence = 1.0-smoothstep(view.interaction_start.w*0.3,view.interaction_start.w+0.6,length(away));
    let lateral = away-up*dot(away,up);
    let push = lateral/max(length(lateral),0.05)*influence*0.55;
    let bend = east*wind + forward*height*0.25 + push;
    let offset = up*(height*p.y*(1.0-influence*0.45)) + side*(p.x*mix(0.07,1.1,far)*fade) + forward*(0.05*fade) + bend*(p.y*p.y)*fade;
    var out: Varying;
    out.relative = root+offset;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative,1.0);
    out.normal = normalize(up*0.7+forward*0.3);
    out.color = in.color_angle.xyz;
    out.tip = p.y;
    out.far = far;
    return out;
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    // Occlusion: a blade stands among its neighbours, so light reaches its
    // tip and little of its root. From afar the field averages out, and dark
    // roots would read as dirt specks.
    let occlusion = mix(0.30, 1.0, smoothstep(0.0, 0.85, in.tip));
    let color = in.color * mix(occlusion, 1.0, in.far) * mix(1.0, 1.18, in.tip);
    return encode(lit(color,normalize(in.normal),0.0,in.relative));
}
