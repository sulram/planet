//! The humanoid: the bones VRM names, and what Mixamo calls them.

/// `(VRM 0.x humanoid name, Mixamo node name without the `mixamorig:` prefix)`.
/// A bone is its index in this table.
pub const BONES: &[(&str, &str)] = &[
    ("hips", "Hips"),
    ("spine", "Spine"),
    ("chest", "Spine1"),
    ("upperChest", "Spine2"),
    ("neck", "Neck"),
    ("head", "Head"),
    ("leftShoulder", "LeftShoulder"),
    ("leftUpperArm", "LeftArm"),
    ("leftLowerArm", "LeftForeArm"),
    ("leftHand", "LeftHand"),
    ("rightShoulder", "RightShoulder"),
    ("rightUpperArm", "RightArm"),
    ("rightLowerArm", "RightForeArm"),
    ("rightHand", "RightHand"),
    ("leftUpperLeg", "LeftUpLeg"),
    ("leftLowerLeg", "LeftLeg"),
    ("leftFoot", "LeftFoot"),
    ("leftToes", "LeftToeBase"),
    ("rightUpperLeg", "RightUpLeg"),
    ("rightLowerLeg", "RightLeg"),
    ("rightFoot", "RightFoot"),
    ("rightToes", "RightToeBase"),
    ("leftThumbProximal", "LeftHandThumb1"),
    ("leftThumbIntermediate", "LeftHandThumb2"),
    ("leftThumbDistal", "LeftHandThumb3"),
    ("leftIndexProximal", "LeftHandIndex1"),
    ("leftIndexIntermediate", "LeftHandIndex2"),
    ("leftIndexDistal", "LeftHandIndex3"),
    ("leftMiddleProximal", "LeftHandMiddle1"),
    ("leftMiddleIntermediate", "LeftHandMiddle2"),
    ("leftMiddleDistal", "LeftHandMiddle3"),
    ("leftRingProximal", "LeftHandRing1"),
    ("leftRingIntermediate", "LeftHandRing2"),
    ("leftRingDistal", "LeftHandRing3"),
    ("leftLittleProximal", "LeftHandPinky1"),
    ("leftLittleIntermediate", "LeftHandPinky2"),
    ("leftLittleDistal", "LeftHandPinky3"),
    ("rightThumbProximal", "RightHandThumb1"),
    ("rightThumbIntermediate", "RightHandThumb2"),
    ("rightThumbDistal", "RightHandThumb3"),
    ("rightIndexProximal", "RightHandIndex1"),
    ("rightIndexIntermediate", "RightHandIndex2"),
    ("rightIndexDistal", "RightHandIndex3"),
    ("rightMiddleProximal", "RightHandMiddle1"),
    ("rightMiddleIntermediate", "RightHandMiddle2"),
    ("rightMiddleDistal", "RightHandMiddle3"),
    ("rightRingProximal", "RightHandRing1"),
    ("rightRingIntermediate", "RightHandRing2"),
    ("rightRingDistal", "RightHandRing3"),
    ("rightLittleProximal", "RightHandPinky1"),
    ("rightLittleIntermediate", "RightHandPinky2"),
    ("rightLittleDistal", "RightHandPinky3"),
];

pub const HIPS: usize = 0;

pub fn from_vrm(name: &str) -> Option<usize> {
    BONES.iter().position(|(vrm, _)| *vrm == name)
}

pub fn from_mixamo(node_name: &str) -> Option<usize> {
    // Exporters spell the namespace `mixamorig:Hips`, `mixamorig_Hips` or drop it.
    let name = node_name
        .strip_prefix("mixamorig:")
        .or_else(|| node_name.strip_prefix("mixamorig_"))
        .unwrap_or(node_name);
    BONES.iter().position(|(_, mixamo)| *mixamo == name)
}
