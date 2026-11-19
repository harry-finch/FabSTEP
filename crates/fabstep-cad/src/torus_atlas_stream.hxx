// OCCT starts every entity definition on a fresh line as "#123 = ENTITY".
// Remove only these two insignificant spaces. Bound the transient prefix
// independently of file size and retain all quoted strings and comments.
class AtlasStepStream : public std::streambuf {
public:
  explicit AtlasStepStream(StepOutputBuffer &bytes) : output(bytes) {}
  bool finish() {
    flush_prefix();
    return flush_bytes();
  }

protected:
  std::streamsize xsputn(const char *input, std::streamsize count) override {
    if (count < 0)
      return 0;
    for (std::streamsize i = 0; i < count; ++i)
      if (!consume(input[i]))
        return i;
    return count;
  }
  int_type overflow(int_type c) override {
    if (traits_type::eq_int_type(c, traits_type::eof()))
      return traits_type::not_eof(c);
    return consume(traits_type::to_char_type(c)) ? c : traits_type::eof();
  }

private:
  StepOutputBuffer &output;
  std::array<char, 8192> buf{};
  size_t used = 0;
  std::array<char, 32> prefix{};
  size_t length = 0;
  bool quoted = false, comment = false, start = true, after_equals = false, failed = false;
  char previous = 0;
  bool flush_bytes() {
    if (used && output.sputn(buf.data(), used) != static_cast<std::streamsize>(used))
      failed = true;
    used = 0;
    return !failed;
  }
  void put(char c) {
    if (used == buf.size() && !flush_bytes())
      return;
    buf[used++] = c;
  }
  void flush_prefix() {
    for (size_t j = 0; j < length; ++j)
      put(prefix[j]);
    length = 0;
  }
  bool consume(char c) {
    if (failed)
      return false;
    if (after_equals) {
      after_equals = false;
      if (c == ' ')
        return true;
    }
    if (length) {
      const bool digit = c >= '0' && c <= '9';
      if (digit && prefix[length - 1] != ' ' && length + 1 < prefix.size()) {
        prefix[length++] = c;
        return true;
      }
      if (c == ' ' && length > 1 && prefix[length - 1] != ' ' && length + 1 < prefix.size()) {
        prefix[length++] = c;
        return true;
      }
      if (c == '=' && length > 2 && prefix[length - 1] == ' ') {
        --length;
        flush_prefix();
        put('=');
        after_equals = true;
        return !failed;
      }
      flush_prefix();
    }
    if (start && !quoted && !comment && c == '#') {
      prefix[length++] = '#';
      start = false;
      return true;
    }
    if (!quoted && !comment && previous == '/' && c == '*')
      comment = true;
    else if (comment && previous == '*' && c == '/')
      comment = false;
    else if (!comment && c == '\'')
      quoted = !quoted;
    previous = c;
    start = (c == '\n' || c == '\r');
    put(c);
    return !failed;
  }
};
