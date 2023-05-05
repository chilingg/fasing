import { createContext } from "react";

const context = await window.__TAURI__.invoke("generate_context");



// export const windowContext = createContext(context.window);