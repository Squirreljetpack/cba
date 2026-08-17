# Bogger Color Mappings and Level Reference

This document summarizes the log levels, ANSI color mappings, formatting modes (`Fg` vs `Bg`), macros, priorities, and verbosity filter levels used by `cba::bog`.

---

## 1. Color and Tag Mappings by Level

| `BogLevel` | Tag Label (`Fg`) | Tag Label (`Bg`) | Foreground Mode (`Fg`) Style | Background Mode (`Bg`) Style | Associated Macro | Description |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`NOTE`** | `NOTE` | `NOTE ` | **Blue** text | **Black** text on **Blue** background | `nbog!` | Notes / informational notes |
| **`ERROR`** | `ERRO` | `ERROR` | **Red** text | **Black** text on **Red** background | `ebog!` | Errors |
| **`WARN`** | `WARN` | `WARN ` | **Yellow** text | **Black** text on **Yellow** background | `wbog!` | Warnings |
| **`_WRN`** | `WARN` | `WARN ` | **Yellow** text | **Black** text on **Yellow** background | `_wbog!` | Low-priority warnings |
| **`INFO`** | `INFO` | `INFO ` | **Green** text | **Black** text on **Green** background | `ibog!` | Informational output |
| **`_NFO`** | `INFO` | `INFO ` | **Green** text | **Black** text on **Green** background | `_ibog!` | Low-priority info |
| **`DEBUG`** | `DBUG` | `DEBUG` | **Cyan** text | **Black** text on **Cyan** background | `dbog!` | Debug messages |
| **`EMPTY`** | *(empty)* | *(empty)* | **Black** text | **Black** text on **White** background | `mbog!` | Untagged / minimal banner |
| **`CUSTOM(s)`** | `<s>` | `<s>` | **Light Cyan** (`BrightCyan`) text | **Black** text on **Light Cyan** (`BrightCyan`) background | `cbog!` | Custom discriminant tag |
| **`___`** | *(none)* | *(none)* | Unstyled | Unstyled | N/A | Lowest / never filter level |

---

## 2. Formatting Render Modes

### Foreground Style (`Fg`) — `init_bogger(true, ...)`
Formats messages with square brackets around the level and optional tag:
- Without tag: `[<LEVEL>] <message>` (e.g. `[INFO] Operation complete`)
- With tag: `[<LEVEL>: <tag>] <message>` (e.g. `[ERRO: 404] Not found`)
- Empty level: `[] <message>` or `[<tag>] <message>`

### Background Style (`Bg`) — `init_bogger(false, ...)`
Formats messages with solid colored background badge blocks:
- Without tag: `<LEVEL> <message>` (e.g. `INFO  Operation complete`)
- With tag: `<LEVEL>| <tag> <message>` (e.g. `ERROR| 404 Not found`)
- Empty level: ` <message>` or `| <tag> <message>`

---

## 3. Level Priorities and Filtering

Messages are filtered based on numeric priority (higher numbers indicate higher severity / permanence):

| Level | Priority Value |
| :--- | :--- |
| `NOTE`, `EMPTY`, `CUSTOM(_)` | **120** |
| `ERROR` | **100** |
| `WARN` | **80** |
| `INFO` | **60** |
| `_WRN`, `_NFO` | **40** |
| `DEBUG` | **20** |
| `___` | **0** |

---

## 4. Verbosity Filter Mapping (`init_filter(verbosity)`)

Calling `init_filter(verbosity)` maps numeric CLI verbosity levels to minimum `BogLevel` thresholds:

| `verbosity` | Minimum Level Emitted | Minimum Priority | Visible Levels |
| :--- | :--- | :--- | :--- |
| **`0`** | *(Silenced)* | `u8::MAX` (255) | None |
| **`1`** | `BogLevel::NOTE` | `120` | `NOTE`, `EMPTY`, `CUSTOM` |
| **`2`** | `BogLevel::ERROR` | `100` | `ERROR`, `NOTE`, `EMPTY`, `CUSTOM` |
| **`3`** | `BogLevel::WARN` | `80` | `WARN`, `ERROR`, `NOTE`, `EMPTY`, `CUSTOM` |
| **`4`** *(Default)* | `BogLevel::INFO` | `60` | `INFO`, `WARN`, `ERROR`, `NOTE`, `EMPTY`, `CUSTOM` |
| **`5`** | `BogLevel::_WRN` | `40` | `_WRN`, `_NFO`, `INFO`, `WARN`, `ERROR`, `NOTE`, `EMPTY`, `CUSTOM` |
| **`6`** | `BogLevel::DEBUG` | `20` | `DEBUG`, `_WRN`, `_NFO`, `INFO`, `WARN`, `ERROR`, `NOTE`, `EMPTY`, `CUSTOM` |
| **`7+`** | `BogLevel::___` | `0` | All messages emitted |
