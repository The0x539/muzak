# indices ? fps sampleRate sampleCount
GetFrameIndices ← (
  ⊙⊸÷ # Duration
  ⌈×  # Frame count
  ÷⟜⇡ # 0..1
  ⁅×  # 0..samples
)

MinMag ← ⨬⋅∘⊙◌ ◡(>∩⌵)

# Find "features" in the audio, currently defined as positive zero-crossings
Features ← ˜⊂0 >⊃↘₋₁↘₁ ±

# offset ? index features radius
🔎 ← ⬚0⊢⊚ ˜↙↘ ↥₀
# offset ? index features radius
🔍 ← ¯⬚0⊢⊚⇌ ˜↘ ⊙(↥₀˜-) ⟜⧻ ↙+₁ ↥₀ # god this was a nightmare

# indices ? indices features radius
🔜 ← +MinMag⤚⊃≡🔍≡🔎 ⊙¤

# indices ? indices audio width
SnapFrameIndices ← ⍜-🔜 ⊃(⋅⋅÷₂|∘|⋅Features|/↥⧈-)

Pad ← ⬚0↙ ˜⊂⊙˜∩∘ ⍜÷₈⌈ ⊸⧋⧻
# frame ? frame height
Encode ← °⋯ ↯∞_¯8 ⇌ Pad

# frame ? height samples
PlotCurve ← ↧⊓⌞⊞≤⊞≥ ⊓⇡(⊃⧈↧⧈↥ ⊂⊸⊢) # ⊃⧈↧⧈↥ is faster than ⧈⊃↧↥

# samples ? position audio width height
🍕 ← ⬚÷₂↙ ⊃(⋅⋅∘|⋅∘|∘) ↘

# frame ? position audio width height
Render ← PlotCurve⤙🍕

# audio ? audio width height
Remap ← ⁅×⊙-₁ ÷⊸/↥ -⊸/↧ ˜⌝↘

# ? fps sampleRate audio width height frameSink
Main ← (
  GetFrameIndices⊙⊙⊸⧻
  ⊸₂SnapFrameIndices
  ⊙⊸₂Remap
  &cl⊸≡(&w Encode ⊸Render) ⊙¤
)

⊙◌ # TODO: make dynamic strobe optional

≡wait≡spawnMain
