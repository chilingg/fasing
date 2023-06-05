import { CloseIcon } from "./Icons";
import { useState, useRef } from "react"
import style from "@/styles/Input.module.css"
import { Button } from "./Button";

export default function Input({ type = "text", label, value, setValue, ...props }) {
    const [focused, setFocused] = useState(false);
    const inputRef = useRef();

    return (
        <>
            {label && <label>{label}</label>}
            <div className={style.inputContainer}>
                <input
                    ref={inputRef}
                    className={style.input}
                    type={type}
                    value={value}
                    onChange={(e) => setValue(e.target.value)}
                    onFocus={() => setFocused(true)}
                    onBlur={() => setFocused(false)}
                    {...props}
                />
                {type === "text" && focused && inputRef.current.value && (
                    <div className={style.clearBtn} onMouseDown={e => e.preventDefault()} onClick={() => setValue("")}>
                        <CloseIcon size={10} pos={1} />
                    </div>
                )}
            </div >
        </>
    )
}
