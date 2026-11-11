┌─╴Util ~ ⧞ ⭹ 🗕
  🗕 ← ⨬∘⋅∘ <◡∩⌵ # "Minimize": Choose the argument with the smaller absolute value
  ⭹ ← ↙↧⊙⊸⧻     # ↙, but accepts a length larger than the array
  ⧞ ← ⨬∘⋅0 =∞⊸⌵ # Replace infinite values with zero
└─╴

# Calculations that take place BEFORE the big loop of rendering each frame.
# Performance is desirable but not critical.
┌─╴Prepare
  # Find "features" in the audio, currently defined as positive zero-crossings
  Features ← ˜⊂0 >⊃↘₋₁↘₁ ±

  # Find the distance to the nearest feature at/after `index` within `radius`
  # offset ? index features radius
  ↝ ← ⬚∞⊢⊚ ˜⭹↘
  # Find the distance to the nearest feature at/before `index` within `radius`
  # offset ? index features radius
  ↜ ← ¯⬚∞⊣⊚ ↘ ⤚(↥₀˜-⧻) ⭹+₁
  # Find the distances to the nearest features in either direction
  # offsets ? indices features radius
  ↭ ← ⧞🗕 ⊃≡↜≡↝ ⊙¤

  # Get evenly-spaced indices into the audio samples based on framerate and duration
  # indices ? fps audio
  GetFrameIndices ← ⁅× ÷⟜⇡ ⌈× ⊙(⊸÷&asr⧻)

  # Adjust frame indices to be centered around the nearest feature in the audio track.
  # If an index is more than 1 frame's worth of motion away
  # from the nearest feature, then that index will not change.
  # indices ? indices audio resolution
  SnapFrameIndices ← + ⍜+⟜↭ ⊃(⁅÷₂⋅⋅⊣|∘|⋅Features|/↥⧈-)

  # Transform any interval to span the height of the output video
  # audio ? audio resolution
  Remap ← ⁅× ⊙(-1⊢) ÷⊸/↥ -⊸/↧

  # indices audio ? fps audio resolution
  Call ← ⊙Remap ⊸₂SnapFrameIndices ⊸GetFrameIndices
└─╴

# The code for actually rendering an individual frame and feeding it to ffmpeg.
# Performance is critical: ideally, this produces data as fast as ffmpeg can consume it.
┌─╴Crunch
  # Take a "slice" of the audio data.
  # If necessary, pad it with "zero" (i.e. video center) to fill the video width.
  # samples ? index audio resolution
  🍕 ← ⬚÷₂↙ ⤚⋅°⊟ ↘

  # Draw a frame by assigning an audio sample to each X coordinate,
  # and marking a pixel as white iff its Y coordinate
  # is between the audio samples from the previous and current column.
  # frame ? samples resolution
  PlotCurve ← ↧⊓⌞⊞≤⊞≥ ˜⊓(⇡⊢|⊃⧈↧⧈↥ ⊂⊸⊢) # ⊃⧈↧⧈↥ is faster than ⧈⊃↧↥

  # Add zeroes to a frame to ensure its dimensions match the expected output resolution,
  # as well as extending each row to be a multiple of 8 pixels.
  # frame ? resolution frame
  Pad ← ⬚0↙ ⍜(÷₈⊣)⌈

  # Convert a boolean array to ffmpeg's monob format
  # packedBits ? bools
  Encode ← °⋯ ↯∞_¯8

  # ? index audio resolution frameSink
  Call ← &w Encode ⇌ ˜Pad ⊸PlotCurve ⊸🍕
└─╴

# ? fps audio resolution frameSink
Main ← ≡⌟₃Crunch ⊸Prepare

⊙◌ # TODO: make dynamic strobe optional

wait≡spawn◇Main
