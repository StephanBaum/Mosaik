import Darwin
import Foundation

/// The boot ID of CRYPTO_SPEC 6.9.2: `sysctlbyname("kern.boottime")` as big-endian `u64 sec || u64 usec`
/// (16 bytes). The core accepts quick unlock after a master-password unlock only while this value is the one
/// stored with the unlock. A read failure gives an empty value, which the core refuses: the password is then
/// required (fail safe).
public enum BootId {
    public static let length = 16

    /// The 16 bytes for a boot time.
    public static func encode(seconds: UInt64, microseconds: UInt64) -> Data {
        var out = Data()
        out.reserveCapacity(length)
        withUnsafeBytes(of: seconds.bigEndian) { out.append(contentsOf: $0) }
        withUnsafeBytes(of: microseconds.bigEndian) { out.append(contentsOf: $0) }
        return out
    }

    /// The boot ID of this boot, or empty when the system call fails.
    public static func current() -> Data {
        var boot = timeval()
        var size = MemoryLayout<timeval>.stride
        let status = sysctlbyname("kern.boottime", &boot, &size, nil, 0)
        guard status == 0, size == MemoryLayout<timeval>.stride, boot.tv_sec > 0, boot.tv_usec >= 0 else {
            return Data()
        }
        return encode(seconds: UInt64(boot.tv_sec), microseconds: UInt64(boot.tv_usec))
    }
}
