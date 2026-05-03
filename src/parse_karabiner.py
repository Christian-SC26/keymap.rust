import json
import os
import re

def format_key(key_str):
    if not key_str:
        return ""
    
    replacements = {
        # --- Модификаторы ---
        "left_command": "L⌘", "right_command": "R⌘", "lcmd": "L⌘", "rcmd": "R⌘", "command": "⌘", "cmd": "⌘",
        "left_option": "L⌥", "right_option": "R⌥", "lalt": "L⌥", "ralt": "R⌥", "lopt": "L⌥", "ropt": "R⌥", "option": "⌥", "alt": "⌥", "opt": "⌥",
        "left_control": "L⌃", "right_control": "R⌃", "lctrl": "L⌃", "rctrl": "R⌃", "control": "⌃", "ctrl": "⌃",
        "left_shift": "L⇧", "right_shift": "R⇧", "lshift": "L⇧", "rshift": "R⇧", "shift": "⇧",
        "hyper": "Hyper",

        # --- Спецклавиши ---
        "vk_none": "-", "return": "↩", "enter": "↩", "space": "␣", "spacebar": "␣",
        "escape": "⎋", "tab": "⇥", "caps_lock": "⇪",
        "delete_or_backspace": "⌫", "delete_forward": "⌦",

        # --- Навигация ---
        "left_arrow": "←", "right_arrow": "→", "up_arrow": "↑", "down_arrow": "↓",
        "page_up": "⇞", "page_down": "⇟", "home": "↖", "end": "↘",

        # --- Знаки препинания ---
        "grave_accent_and_tilde": "ˋ", "hyphen": "-", "equal_sign": "=",
        "open_bracket": "[", "close_bracket": "]", "backslash": "\\",
        "semicolon": ";", "quote": "'", "comma": ",", "period": ".", "slash": "/",

        # --- Hex-коды (skhd) ---
        "0x32": "ˋ", "0x1b": "-", "0x18": "=",
        "0x21": "[", "0x1e": "]", "0x2a": "\\",
        "0x29": ";", "0x27": "'", "0x2b": ",", "0x2f": ".", "0x2c": "/",

        # --- Медиа и F-клавиши ---
        "play_or_pause": "⏯", "mute": "🔇", "volume_decrement": "🔉", "volume_increment": "🔊",
        "display_brightness_decrement": "🔅", "display_brightness_increment": "🔆",
        "mission_control": "Mission Control", "launchpad": "Launchpad",
        "f1": "F1", "f2": "F2", "f3": "F3", "f4": "F4", "f5": "F5", "f6": "F6",
        "f7": "F7", "f8": "F8", "f9": "F9", "f10": "F10", "f11": "F11", "f12": "F12"
    }
    
    key_str = key_str.lower()
    return replacements.get(key_str, key_str)

def process_modifiers(mods_list):
    if not mods_list:
        return ""

    bases = set()
    for m in mods_list:
        if '⌘' in m: bases.add('cmd')
        elif '⌥' in m: bases.add('opt')
        elif '⌃' in m: bases.add('ctrl')
        elif '⇧' in m: bases.add('shift')
        elif m.lower() == 'hyper': bases.update(['cmd', 'opt', 'ctrl', 'shift'])
    
    if {'cmd', 'opt', 'ctrl', 'shift'}.issubset(bases):
        return "Hyper"
    
    def weight(m):
        if '⌘' in m: return 1
        if '⌥' in m: return 2
        if '⌃' in m: return 3
        if '⇧' in m: return 4
        return 5 
        
    mods_list.sort(key=lambda m: (weight(m), m))
    return " + ".join(mods_list)

def add_emacs_shortcuts(shortcuts_dict):
    emacs_bindings = [
        (["ctrl"], "a", "Начало абзаца/строки"),
        (["ctrl"], "e", "Конец абзаца/строки"),
        (["ctrl"], "f", "Вперед на один символ"),
        (["ctrl"], "b", "Назад на один символ"),
        (["ctrl"], "n", "Вниз на одну строку"),
        (["ctrl"], "p", "Вверх на одну строку"),
        (["ctrl"], "d", "Удалить символ спереди (Delete)"),
        (["ctrl"], "h", "Удалить символ сзади (Backspace)"),
        (["ctrl"], "k", "Удалить текст до конца абзаца"),
        (["ctrl"], "y", "Вставить вырезанный текст (Yank)"),
        (["ctrl"], "o", "Вставить новую строку после курсора"),
        (["ctrl"], "t", "Поменять местами соседние символы"),
        (["ctrl"], "v", "На страницу вниз")
    ]
    
    for mods, key, desc in emacs_bindings:
        formatted_mods = [format_key(m) for m in mods]
        trigger = f"{process_modifiers(formatted_mods)} + {format_key(key)}"
        add_to_dict(shortcuts_dict, "sy", trigger, "-", desc)

def format_description(desc):
    if not desc or desc == "-":
        return desc
    
    # Для консольного UI просто оставляем текст чистым, без markdown разметки
    desc = re.sub(r'^([a-zA-Zа-яА-Я0-9\s_-]+)\s*:', r'\1: ', desc, count=1)
    desc = re.sub(r'(?i)\(\s*disabled in:\s*([^)]+)\)', r' | Disabled in: \1', desc)
    desc = re.sub(r'(?i)(?<!\*\*)disabled in:', r' | Disabled in: ', desc)
    
    return desc.strip()

def parse_karabiner_json(file_path, shortcuts_dict):
    file_path = os.path.expanduser(file_path)
    if not os.path.exists(file_path):
        print(f"[Warn] Karabiner config не найден: {file_path}")
        return

    with open(file_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    profiles = data.get("profiles", [])
    for profile in profiles:
        rules = profile.get("complex_modifications", {}).get("rules", [])
        for rule in rules:
            raw_description = rule.get("description", "-")
            description = format_description(raw_description)

            for manipulator in rule.get("manipulators", []):
                if manipulator.get("type") != "basic":
                    continue

                from_data = manipulator.get("from", {})
                key = format_key(from_data.get("key_code") or from_data.get("consumer_key_code") or from_data.get("pointing_button") or from_data.get("any") or "")
                
                mandatory_mods = from_data.get("modifiers", {}).get("mandatory", [])
                
                if isinstance(mandatory_mods, list):
                    mods_list = [format_key(m) for m in mandatory_mods]
                    mods = process_modifiers(mods_list)
                else:
                    mods = ""
                
                trigger = f"{mods} + {key}" if mods and key else key or mods
                if not trigger: trigger = "-"

                to_data = manipulator.get("to", [])
                
                if trigger == "-" and not to_data:
                    continue

                actions = []
                for t in to_data:
                    if "key_code" in t or "consumer_key_code" in t:
                        t_key = format_key(t.get("key_code") or t.get("consumer_key_code"))
                        to_mods = t.get("modifiers", [])
                        if isinstance(to_mods, list) and to_mods:
                            t_mods_list = [format_key(m) for m in to_mods]
                            actions.append(f"{process_modifiers(t_mods_list)} + {t_key}")
                        else:
                            actions.append(f"{t_key}")
                    elif "shell_command" in t:
                        actions.append(f"{t['shell_command']}")
                    elif "set_variable" in t:
                        actions.append(f"var: {t['set_variable'].get('name')}")
                
                action_str = " ".join(actions) if actions else "-"
                add_to_dict(shortcuts_dict, "ke", trigger, action_str, description)

def parse_skhd_trigger(trigger_raw):
    trigger_parts = re.split(r'\s*\+\s*|\s*-\s*', trigger_raw)
    formatted_parts = [format_key(p.strip()) for p in trigger_parts if p.strip()]

    if len(formatted_parts) > 1:
        mods_list = formatted_parts[:-1]
        key = formatted_parts[-1]
        mods = process_modifiers(mods_list)
        return f"{mods} + {key}" if mods else key
    else:
        return formatted_parts[0] if formatted_parts else "-"

def parse_skhd_config(file_path, shortcuts_dict):
    file_path = os.path.expanduser(file_path)
    if not os.path.exists(file_path):
        print(f"[Warn] skhd config не найден: {file_path}")
        return

    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    in_block = False
    current_trigger = "-"
    block_actions = []

    for line in lines:
        line = line.strip()
        if not line or line.startswith("#"):
            continue

        if line.endswith("["):
            in_block = True
            trigger_raw = line[:-1].strip()
            trigger_raw = re.sub(r'(:|->)$', '', trigger_raw).strip()
            current_trigger = parse_skhd_trigger(trigger_raw)
            block_actions = []
            continue

        if in_block:
            if line == "]":
                in_block = False
                if block_actions:
                    description = " | ".join(block_actions)
                    add_to_dict(shortcuts_dict, "sk", current_trigger, "-", description)
                continue

            if line.endswith("~"):
                app_raw = line[:-1].strip()
                if app_raw == "*":
                     block_actions.append("Остальные: pass-through (~)")
                else:
                     app_name = app_raw.strip('\"\'')
                     block_actions.append(f"{app_name}: pass-through (~)")
                continue

            if ":" in line or "->" in line:
                separator = ":" if ":" in line else "->"
                parts = line.split(separator, 1)
                app_raw = parts[0].strip()
                action_raw = parts[1].strip()

                match = re.search(r'(--.+)', action_raw)
                action_desc = match.group(1).strip() if match else action_raw

                if app_raw == "*":
                    block_actions.append(f"Остальные: {action_desc}")
                else:
                    app_name = app_raw.strip('\"\'')
                    block_actions.append(f"{app_name}: {action_desc}")
            continue

        if ":" not in line and "->" not in line:
            continue

        separator = ":" if ":" in line else "->"
        parts = line.split(separator, 1)
        trigger_raw = parts[0].strip()
        action_raw = parts[1].strip()

        trigger = parse_skhd_trigger(trigger_raw)

        match = re.search(r'(--.+)', action_raw)
        if match:
            description = match.group(1).strip()
        else:
            description = action_raw

        add_to_dict(shortcuts_dict, "sk", trigger, "-", description)

def add_to_dict(shortcuts_dict, source, trigger, action, description):
    if trigger not in shortcuts_dict:
        shortcuts_dict[trigger] = {"sources": set(), "actions": [], "descriptions": []}
    
    shortcuts_dict[trigger]["sources"].add(source)
    
    if action and action != "-" and action not in shortcuts_dict[trigger]["actions"]:
        shortcuts_dict[trigger]["actions"].append(action)
    
    if description and description != "-" and description not in shortcuts_dict[trigger]["descriptions"]:
        shortcuts_dict[trigger]["descriptions"].append(description)

def export_json(shortcuts_dict, output_path):
    output_path = os.path.expanduser(output_path)
    output_data = []
    
    for trigger, data in shortcuts_dict.items():
        sources_str = ", ".join(sorted(list(data["sources"])))
        
        actions_str = " | ".join(data["actions"]).replace("`", "") if data["actions"] else "-"
        descriptions_str = " | ".join(data["descriptions"])
        descriptions_str = descriptions_str.replace("**", "").replace("<br>", " ") if descriptions_str else "-"
        
        # Подготавливаем ключи для подсветки в Rust
        keys = [k.strip() for k in trigger.split("+")] if trigger != "-" else []

        output_data.append({
            "source": sources_str,
            "trigger": trigger,
            "keys": keys,
            "action": actions_str,
            "desc": descriptions_str
        })

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(output_data, f, ensure_ascii=False, indent=2)
    print(f"Успешно! JSON сохранен по пути: {output_path}")

# ==========================================
# КОНФИГИ И ЗАПУСК
# ==========================================

all_shortcuts = {}

add_emacs_shortcuts(all_shortcuts)
parse_karabiner_json("~/.config/karabiner/karabiner.json", all_shortcuts)
parse_skhd_config("~/.skhdrc", all_shortcuts)

output_json_path = "~/.config/karabiner/shortcuts.json"
export_json(all_shortcuts, output_json_path)
