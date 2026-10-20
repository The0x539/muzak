# indices ? fps sampleRate sampleCount
GetFrameIndices ← (
  ⊙⊸÷ # Duration
  ⌈×  # Frame count
  ÷⟜⇡ # 0..1
  ⁅×  # 0..samples
)

# Find "features" in the audio, currently defined as positive zero-crossings
Features ← ˜⊂0 >⊃↘₋₁↘₁ ±

# indices ? indices audio
SnapFrameIndices ← (
  ⊙Features
  # TODO: Search in both directions
  + ⟜≡⌟(⬚0⊢⊚↙800↘)
)

Pad    ← ⬚0↙ ⊂100 ⍜÷₈⌈⊣⊸△
Encode ← °⋯ ↯∞_¯8 ⇌ Pad

# frame ? height samples
PlotCurve ← ↧⊓⌞⊞≤⊞≥ ⊓⇡(⊃⧈↧⧈↥ ⊂⊸⊢) # ⊃⧈↧⧈↥ is faster than ⧈⊃↧↥

# frame ? position audio
Render ← PlotCurve 100 ⬚50↙800↘

Remap ← ⁅×-₁100 ÷⊸/↥ -⊸/↧ ⌝↘800

# ? fps sampleRate audio frameSink
Main ← (
  GetFrameIndices⊙⊙⊸⧻
  ⍜-⊸SnapFrameIndices ÷₂800
  ⊙Remap
  &cl⊸≡⌟₂(&w Encode Render)
)

⊙◌ # TODO: make dynamic strobe optional

≡wait≡spawnMain
