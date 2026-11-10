//! Platform-independent Python standard-library module roots by target release.
//!
//! Data comes from CPython's generated `Python/stdlib_module_names.h` at
//! v3.10.19, v3.11.14, v3.12.12, v3.13.5, v3.14.0 and v3.15.0. That
//! file creates `sys.stdlib_module_names`. Python documents this set as
//! platform-independent, including native modules unavailable on the host.
//! This is the language catalog, not Ruff's broader internal/test-module list.
//! Python 3.7–3.9 do not have this authoritative API and remain unverified.

pub(super) const COMMON: &str = concat!(
    "__future__|_abc|_aix_support|_ast|_asyncio|_bisect|_blake2|_bz2|_codecs|_codecs_cn|",
    "_codecs_hk|_codecs_iso2022|_codecs_jp|_codecs_kr|_codecs_tw|_collections|_collections_abc|",
    "_compat_pickle|_contextvars|_csv|_ctypes|_curses|_curses_panel|_datetime|_dbm|_decimal|",
    "_elementtree|_frozen_importlib|_frozen_importlib_external|_functools|_gdbm|_hashlib|_heapq|",
    "_imp|_io|_json|_locale|_lsprof|_lzma|_markupbase|_md5|_multibytecodec|_multiprocessing|",
    "_opcode|_operator|_osx_support|_overlapped|_pickle|_posixshmem|_posixsubprocess|_py_abc|",
    "_pydecimal|_pyio|_queue|_random|_scproxy|_sha1|_sha3|_signal|_sitebuiltins|_socket|",
    "_sqlite3|_sre|_ssl|_stat|_statistics|_string|_strptime|_struct|_symtable|_thread|",
    "_threading_local|_tkinter|_tracemalloc|_uuid|_warnings|_weakref|_weakrefset|_winapi|",
    "_zoneinfo|abc|antigravity|argparse|array|ast|asyncio|atexit|base64|bdb|binascii|bisect|",
    "builtins|bz2|cProfile|calendar|cmath|cmd|code|codecs|codeop|collections|colorsys|",
    "compileall|concurrent|configparser|contextlib|contextvars|copy|copyreg|csv|ctypes|curses|",
    "dataclasses|datetime|dbm|decimal|difflib|dis|doctest|email|encodings|ensurepip|enum|errno|",
    "faulthandler|fcntl|filecmp|fileinput|fnmatch|fractions|ftplib|functools|gc|genericpath|",
    "getopt|getpass|gettext|glob|graphlib|grp|gzip|hashlib|heapq|hmac|html|http|idlelib|imaplib|",
    "importlib|inspect|io|ipaddress|itertools|json|keyword|linecache|locale|logging|lzma|",
    "mailbox|marshal|math|mimetypes|mmap|modulefinder|msvcrt|multiprocessing|netrc|nt|ntpath|",
    "nturl2path|numbers|opcode|operator|optparse|os|pathlib|pdb|pickle|pickletools|pkgutil|",
    "platform|plistlib|poplib|posix|posixpath|pprint|profile|pstats|pty|pwd|py_compile|pyclbr|",
    "pydoc|pydoc_data|pyexpat|queue|quopri|random|re|readline|reprlib|resource|rlcompleter|",
    "runpy|sched|secrets|select|selectors|shelve|shlex|shutil|signal|site|smtplib|socket|",
    "socketserver|sqlite3|ssl|stat|statistics|string|stringprep|struct|subprocess|symtable|sys|",
    "sysconfig|syslog|tabnanny|tarfile|tempfile|termios|textwrap|this|threading|time|timeit|",
    "tkinter|token|tokenize|trace|traceback|tracemalloc|tty|turtle|turtledemo|types|typing|",
    "unicodedata|unittest|urllib|uuid|venv|warnings|wave|weakref|webbrowser|winreg|winsound|",
    "wsgiref|xml|xmlrpc|zipapp|zipfile|zipimport|zlib|zoneinfo|",
);

pub(super) const PY3_10: &str = concat!(
    "_bootsubprocess|_compression|_crypt|_msi|_sha256|_sha512|aifc|asynchat|asyncore|audioop|",
    "binhex|cgi|cgitb|chunk|crypt|distutils|imghdr|imp|lib2to3|mailcap|msilib|nis|nntplib|",
    "ossaudiodev|pipes|smtpd|sndhdr|spwd|sre_compile|sre_constants|sre_parse|sunau|telnetlib|uu|",
    "xdrlib|",
);

pub(super) const PY3_11: &str = concat!(
    "_bootsubprocess|_compression|_crypt|_msi|_sha256|_sha512|_tokenize|_typing|aifc|asynchat|",
    "asyncore|audioop|cgi|cgitb|chunk|crypt|distutils|imghdr|imp|lib2to3|mailcap|msilib|nis|",
    "nntplib|ossaudiodev|pipes|smtpd|sndhdr|spwd|sre_compile|sre_constants|sre_parse|sunau|",
    "telnetlib|tomllib|uu|xdrlib|",
);

pub(super) const PY3_12: &str = concat!(
    "_compression|_crypt|_msi|_pydatetime|_pylong|_sha2|_tokenize|_typing|_wmi|aifc|audioop|cgi|",
    "cgitb|chunk|crypt|imghdr|lib2to3|mailcap|msilib|nis|nntplib|ossaudiodev|pipes|sndhdr|spwd|",
    "sre_compile|sre_constants|sre_parse|sunau|telnetlib|tomllib|uu|xdrlib|",
);

pub(super) const PY3_13: &str = concat!(
    "_android_support|_apple_support|_colorize|_compression|_interpchannels|_interpqueues|",
    "_interpreters|_ios_support|_opcode_metadata|_pydatetime|_pylong|_pyrepl|_sha2|_suggestions|",
    "_sysconfig|_tokenize|_typing|_wmi|sre_compile|sre_constants|sre_parse|tomllib|",
);

pub(super) const PY3_14: &str = concat!(
    "_android_support|_apple_support|_ast_unparse|_colorize|_hmac|_interpchannels|_interpqueues|",
    "_interpreters|_ios_support|_opcode_metadata|_py_warnings|_pydatetime|_pylong|_pyrepl|",
    "_remote_debugging|_sha2|_suggestions|_sysconfig|_tokenize|_types|_typing|_wmi|_zstd|",
    "annotationlib|compression|sre_compile|sre_constants|sre_parse|tomllib|",
);

pub(super) const PY3_15: &str = concat!(
    "_android_support|_apple_support|_ast_unparse|_colorize|_hmac|_interpchannels|_interpqueues|",
    "_interpreters|_ios_support|_math_integer|_opcode_metadata|_py_warnings|_pydatetime|_pylong|",
    "_pyrepl|_remote_debugging|_sha2|_suggestions|_sysconfig|_tokenize|_types|_typing|_wmi|",
    "_zstd|annotationlib|compression|profiling|tomllib|",
);
