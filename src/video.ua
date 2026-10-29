# Experimental!

# indices ? fps sampleRate sampleCount
GetFrameIndices ← (
  ⊙⊸÷ # Duration
  ⌈×  # Frame count
  ÷⟜⇡ # 0..1
  ⁅×  # 0..samples
)

# Choose the argument with the smaller absolute value
MinMag ← ⨬∘⋅∘ <◡∩⌵

# Find "features" in the audio, currently defined as positive zero-crossings
Features ← ˜⊂0 >⊃↘₋₁↘₁ ±

# ↙, but accepts a length larger than the array
🪚 ← ↙↧⊙⊸⧻

# Find the distance to the nearest `feature` at/after `index`, up to `radius`.
# offset ? index features radius
🔎 ← ⬚∞⊢⊚ ˜🪚↘
# Find the distance to the nearest `feature` at/before `index`, up to `radius`.
# offset ? index features radius
🔍 ← ¯⬚∞⊣⊚ ˜↘ ⊙(↥₀˜-)⟜⧻ 🪚+₁

InfToZero ← ⨬∘⋅0 =∞⊸⌵

# indices ? indices features radius
🔜 ← + InfToZero MinMag ⤚⊃≡🔍≡🔎 ⊙¤

# indices ? indices audio width
SnapFrameIndices ← ⍜+🔜 ⊃(⋅⋅÷₂|∘|⋅Features|÷₂/↥⧈-)

# Add zeroes to a frame to ensure its dimensions are height×8ℤ
# frame ? frame height
Pad ← ⬚0↙ ⊂ ⊙(⍜÷₈⌈ ⊸⧻₁)
# frame ? frame height
Encode ← °⋯ ↯∞_¯8 ⇌ ˜Pad

# frame ? height samples
PlotCurve ← ↧⊓⌞⊞≤⊞≥ ⊓⇡(⊃⧈↧⧈↥ ⊂⊸⊢) # ⊃⧈↧⧈↥ is faster than ⧈⊃↧↥

# samples ? position audio width height
🍕 ← ⬚÷₂↙ ⊃(⋅⋅∘|⋅∘|∘) ↘

# frame ? position audio width height
Render ← PlotCurve⤙🍕

# audio ? audio height
Remap ← ⁅×⊙-₁ ÷⊸/↥ -⊸/↧

# ? fps sampleRate audio width height frameSink
Main ← (
  GetFrameIndices⊙⊙⊸⧻
  ⊸₂SnapFrameIndices
  ⊙⊸₂∪Remap
  ≡(&w Encode ⊸Render) ⊙¤
)

⊙◌ # TODO: make dynamic strobe optional

wait≡spawn◇Main
