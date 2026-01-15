# LIBS - High Performance Minecraft Mod

![LIBS Engine](https://img.shields.io/badge/LIBS-Performance%20Mod-blue?style=for-the-badge)
![License](https://img.shields.io/badge/License-MIT-yellow?style=for-the-badge)

**LIBS (Lightweight Integrated Booster System)** is a performance optimization mod for Minecraft that uses a hybrid Java/Rust architecture to deliver significant FPS improvements.

---

## 📦 Available Versions

| Version | Minecraft | Platform | Status |
|---------|-----------|----------|--------|
| 1.0.0-alpha | 1.21.1 | NeoForge | ✅ Released |
| 1.0.0-alpha | 1.8.9 | Forge | ✅ Released |

---

## 🚀 Features

- **Vulkan/OpenGL Hybrid Renderer** - Hardware-accelerated rendering with automatic fallback
- **Parallel Chunk Processing** - Multi-threaded chunk updates using all CPU cores
- **Off-Heap Memory Management** - Reduces GC pauses for smoother gameplay
- **Optimized Entity System** - Data-oriented ECS architecture
- **LOD System** - Automatic level-of-detail for distant terrain
- **Native Rust Core** - Critical paths optimized in Rust

---

## 🔧 Installation

1. Download the JAR for your Minecraft version from [Modrinth](https://modrinth.com/mod/libs)
2. Place the JAR in your `mods` folder
3. Launch with Forge/NeoForge

### Requirements
- **1.8.9**: Java 8, Forge
- **1.21.1**: Java 21, NeoForge

---

## 🛠️ Building from Source

```bash
# Clone repository
git clone https://github.com/alexsandro1234567/LIBS-Mod.git
cd LIBS-Mod

# Build Rust native library
cd rust-core
cargo build --release
cd ..

# Build 1.8.9 Forge
cd forge-1.8.9-standalone
./gradlew build

# Build 1.21.1 NeoForge
cd ../forge-1.21.1
../gradlew build
```

### Output
- `forge-1.8.9-standalone/build/libs/*.jar`
- `forge-1.21.1/build/libs/*.jar`

---

## 📁 Project Structure

```
LIBS-Mod/
├── common/                 # Shared Java code
├── rust-core/              # Native Rust engine
├── forge-1.8.9-standalone/ # Forge 1.8.9
└── forge-1.21.1/           # NeoForge 1.21.1
```

---

## 📜 License

MIT License - see [LICENSE](LICENSE)

---

## 👨‍💻 Author

**Aiblox** (Alexsandro Alves de Oliveira)
