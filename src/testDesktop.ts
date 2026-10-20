import {invoke} from "@tauri-apps/api/core";


export async function testDesktop() {
    try {
        const desktop = await invoke("get_desktop_info")
        console.log("Escritorio actual: ", desktop)
    } catch (err) {
        console.log("Error: ", err);
    }
}

