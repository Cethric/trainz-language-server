#[tracing::instrument]
pub fn get_wiki_kind_name(kind_name: &str) -> String {
    let mapping = [
        ("achievement-category", "Achievement-category"),
        ("achievement-group", "Achievement-group"),
        ("asset-group", "Asset-group"),
        ("behavior", "Behavior"),
        ("behavior-template", "Behavior-Template"),
        ("bogey", "Bogey"),
        ("buildable", "Buildable"),
        ("consist", "Consist"),
        ("controlset", "Controlset"),
        ("drivercharacter", "Drivercharacter"),
        ("drivercommand", "Drivercommand"),
        ("engine", "Engine"),
        ("enginesound", "Enginesound"),
        ("environment", "Environment"),
        ("fixedtrack", "Fixedtrack"),
        ("gameplaymenu", "Gameplaymenu"),
        ("groundbrush", "Groundbrush"),
        ("groundtexture", "Groundtexture"),
        ("hornsound", "Hornsound"),
        ("html-asset", "HTML-asset"),
        ("industry", "Industry"),
        ("interlocking-tower", "Interlocking-Tower"),
        ("interior", "Interior"),
        ("library", "Library"),
        ("map", "Map"),
        ("mesh", "Mesh"),
        ("mocrossing", "MOCrossing"),
        ("mojunction", "MOJunction"),
        ("mosignal", "MOSignal"),
        ("mospeedboard", "MOSpeedboard"),
        ("pantograph", "Pantograph"),
        ("procedural track", "Procedural track"),
        ("product", "Product"),
        ("product-category", "Product-category"),
        ("profile", "Profile"),
        ("region", "Region"),
        ("scenery", "Scenery"),
        ("scenery-trackside", "Scenery-trackside"),
        ("scenerywithtrack", "SceneryWithTrack"),
        ("tni-physics-plugin", "tni-physics-plugin"),
        ("trainbasespec", "TrainBaseSpec"),
    ];

    for (lower, correct) in mapping {
        if kind_name.eq_ignore_ascii_case(lower) {
            return correct.to_string();
        }
    }

    kind_name.to_string()
}

#[tracing::instrument]
pub fn get_wiki_container_name(container_name: &str) -> String {
    let mapping = [
        ("achievements", "Achievements"),
        ("attached-splines", "attached-splines"),
        ("attached-trigger", "attached-trigger"),
        ("attached-track", "Attached-track"),
        ("bogeys", "Bogeys"),
        ("cameralist", "cameralist"),
        ("consists", "Consists"),
        ("controller-device-list", "Controller-device-list"),
        ("controls", "Controls"),
        ("decal", "Decal"),
        ("driver-settings", "Driver-settings"),
        ("dynamic-brake", "Dynamic-brake"),
        ("extensions", "Extensions"),
        ("flowsize", "Flowsize"),
        ("hardware-controls-list", "Hardware-controls-list"),
        ("inputs", "Inputs"),
        ("junction-vertices", "junction-vertices"),
        ("kuid-table", "Kuid-table"),
        ("levels", "Levels"),
        ("lights", "Lights"),
        ("mass", "Mass"),
        ("member-of-groups", "Member-of-groups"),
        ("mesh-table", "mesh-table"),
        ("mutexes", "mutexes"),
        ("motor", "Motor"),
        ("obsolete-table", "Obsolete-table"),
        ("outputs", "Outputs"),
        ("pressure", "Pressure"),
        ("processes-element", "processes-element"),
        ("privileges", "privileges"),
        ("processes", "Processes"),
        ("queues", "Queues"),
        ("rule-properties", "Rule-properties"),
        ("script-include-table", "script-include-table"),
        ("season-selector", "season-selector"),
        ("signals", "Signals"),
        ("smoke", "Smoke"),
        ("steam", "Steam"),
        ("socket-template-list", "Socket-template-list"),
        ("soundscript", "Soundscript"),
        ("string-table", "String-table"),
        ("texture-variants", "texture-variants"),
        ("track-lod-tree", "track-lod-tree"),
        ("track-sound", "track-sound"),
        ("template-properties", "Template-properties"),
        ("thumbnails", "Thumbnails"),
        ("vertices", "Vertices"),
    ];

    for (lower, correct) in mapping {
        if container_name.eq_ignore_ascii_case(lower) {
            return correct.to_string();
        }
    }

    container_name.to_string()
}
