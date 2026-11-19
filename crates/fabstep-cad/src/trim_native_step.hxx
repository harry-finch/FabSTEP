class StepOutputBuffer : public std::streambuf {
public:
  explicit StepOutputBuffer(size_t limit = step_document_limit) : limit(limit) {}
  const unsigned char *data() const { return bytes.get(); }
  size_t size() const { return written; }
  bool limit_reached() const { return exceeded; }
  unsigned char *release() { return bytes.release(); }

protected:
  std::streamsize xsputn(const char *source, std::streamsize count) override {
    if (count < 0 || static_cast<size_t>(count) > limit - written) {
      exceeded = true;
      return 0;
    }
    if (count == 0) {
      return 0;
    }
    const auto required = written + static_cast<size_t>(count);
    if (required > capacity) {
      capacity = std::min(limit, std::max({required, capacity * 2, size_t{4096}}));
      std::unique_ptr<unsigned char[]> next(new unsigned char[capacity]);
      if (written != 0) {
        std::memcpy(next.get(), bytes.get(), written);
      }
      bytes = std::move(next);
    }
    std::memcpy(bytes.get() + written, source, static_cast<size_t>(count));
    written = required;
    return count;
  }

  int_type overflow(int_type value) override {
    if (traits_type::eq_int_type(value, traits_type::eof())) {
      return traits_type::not_eof(value);
    }
    const char byte = traits_type::to_char_type(value);
    return xsputn(&byte, 1) == 1 ? value : traits_type::eof();
  }

private:
  std::unique_ptr<unsigned char[]> bytes;
  const size_t limit;
  size_t written = 0;
  size_t capacity = 0;
  bool exceeded = false;
};

#include "torus_atlas_stream.hxx"

// Serialize one complete root set. OCCT's compact representation keeps the
// 3D edge curves and surface supports; the reader recomputes the optional
// parametric curves. The caller must validate the complete reimport as usual.
template <class Transfer>
std::unique_ptr<StepOutputBuffer> write_bounded_step(Transfer transfer, size_t limit = step_document_limit, bool dense = false) {
  for (bool compact : {false, true}) {
    STEPControl_Writer writer;
    transfer(writer, compact);
    auto buffer = std::make_unique<StepOutputBuffer>(limit);
    AtlasStepStream leaders(*buffer);
    std::ostream stream(dense ? static_cast<std::streambuf*>(&leaders) : buffer.get());
    const auto status = writer.WriteStream(stream);
    const bool finished = !dense || leaders.finish();
    if (buffer->limit_reached()) {
      if (!compact)
        continue;
      throw CadFailure{document_too_large, "Le document STEP exporté dépasse 128 Mio."};
    }
    if (!finished || status != IFSelect_RetDone || !stream.good() || buffer->size() == 0)
      throw CadFailure{write_failed, "Impossible de sérialiser le document STEP."};
    return buffer;
  }
  throw CadFailure{write_failed, "Impossible de sérialiser le document STEP."};
}
