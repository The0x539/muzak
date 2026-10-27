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

# Find the distance to the nearest `feature` at/after `index`, up to `radius`.
# TODO: Return NaN or inf on failure. Convert to 0 *after* passing through MinMag.
# Reason: The current behavior produces a bug where if probing in ONE direction fails,
# then the successful search in the other direction gets discarded.
# offset ? index features radius
🔎 ← ⬚0⊢⊚ ˜↙↘ ↥₀
# Find the distance to the nearest `feature` at/before `index`, up to `radius`.
# offset ? index features radius
🔍 ← ¯⬚0⊢⊚⇌ ˜↘ ⊙(↥₀˜-) ⟜⧻ ↙+₁ ↥₀ # god this was a nightmare

# indices ? indices features radius
🔜 ← +MinMag⤚⊃≡🔍≡🔎 ⊙¤

# indices ? indices audio width
SnapFrameIndices ← ⍜-🔜 ⊃(⋅⋅÷₂|∘|⋅Features|/↥⧈-)

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

wait≡spawnMain
