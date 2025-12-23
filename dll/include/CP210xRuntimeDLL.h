#ifndef	HOST_LIB_RUNTIME_INCLUDE_CP210XRUNTIMEDLL_H_INCLUDED_QU4N7WC20W
#define	HOST_LIB_RUNTIME_INCLUDE_CP210XRUNTIMEDLL_H_INCLUDED_QU4N7WC20W

/// @addtogroup runtime CP201x Runtime Library
///
/// The Silicon Labs CP210x USB-to-UART bridges are devices that communicate over
/// the Universal Serial Bus(USB) to perform Universal Asynchronous Receiver / Transmitter
/// (UART) data transfers. These devices have many programmable options that can
/// be configured via USB. These devices also often include flexible GPIO functions that
/// can be configured and accessed via USB during runtime. Silicon Labs provides libraries
/// that can be used to configure these devices and access their GPIOs.
///
/// Two interface library files can be used to interface with CP210x devices, CP210xManufacturing.DLL and .LIB, which is responsible for reading and writing
/// the device settings, and CP210xRuntime.DLL and .LIB, which is responsible for interfacing with the device's GPIOs. The APIs for CP210xRuntime are
/// described below.
///
/// This document supercedes, obsoletes and takes precedences over AN223 and AN978.
///
/// The CP210x Runtime Host API provides access to the GPIO port latch, and the DLL is meant for distribution with the product containing a CP210x
/// device.  Please provide feedback to Silicon Labs if you require the static-linked (I.e. linked at build time) verison of this device interface library.
///
/// The CP210x Runtime Host API is provided in the form of a Windows Dynamic Link Library (DLL), CP210xRuntime.DLL and as a Windows
/// static link library (.static.LIB). The host interface
/// DLL communicates with the bridge controller device via the provided device driver and the operating system's USB stack.
///
/// Custom applications can use the CP210x Runtime API implemented in CP210xRuntime.DLL. To use functions implemented in CP210xRuntime.DLL link CP210xRuntime.LIB
/// include CP210xRuntimeDLL.h into any source code files that call any functions implemented in CP210xRuntime.DLL and include the PATH to the location of CP210xRuntime.DLL
/// in your custom application Visual Studio .VCXPROJ Profile File's Properties->Linker->General->Additional Library Directories property.
///
/// Typically, the user initiates communication with the target CP210x device by making a call to @ref CP210xRT_GetNumDevices(). This call
/// returns the number of CP210x target devices. 
/// A handle to the device must first be opened by a call to @ref CP210xRT_Open() using an index determined from the call to @ref CP210xRT_GetNumDevices().
/// The handle will be used for all subsequent accesses. When I/O operations are complete, the device handle is closed by a call to
/// @ref CP210xRT_Close().
///
///	@note: Function calls into this API are blocked until completed. This can take several milliseconds depending on USB traffic.
///
/// @{

#include	<stdint.h>	// For C99 fixed sized types
#include	<stdbool.h>	// For C99 _Bool

#include	"silabs_defs.h"
#include	"silabs_sal.h"

#ifdef _WIN32
// The following ifdef block is the standard way of creating macros which make exporting
// from a DLL simpler. All files within this DLL are compiled with the CP210xRUNTIMEDLL_EXPORTS
// symbol defined on the command line. this symbol should not be defined on any project
// that uses this DLL. This way any other project whose source files include this file see
// CP210xRUNTIMEDLL_API functions as being imported from a DLL, wheras this DLL sees symbols
// defined with this macro as being exported.
#ifdef CP210xRUNTIMEDLL_EXPORTS
#define CP210xRUNTIMEDLL_API
#else
#if defined(SILABS_STATIC_LINK)
#define CP210xRUNTIMEDLL_API extern
#pragma comment (lib, "CP210xRuntime.static.lib")
#else // defined(SILABS_STATIC_LINK)
#define CP210xRUNTIMEDLL_API __declspec(dllimport)
#pragma comment (lib, "CP210xRuntime.lib")
#endif // defined(SILABS_STATIC_LINK)
#endif
#else // !_WIN32
#define CP210xDLL_API
#define WINAPI
#define CP210xRUNTIMEDLL_API
#endif // !_WIN32


/// PartNums as returned by @ref CP210xRT_GetPartNumber()  (@deprecated {instead use @ref SILABS_PARTNUM_CPXXXX})
/// @defgroup RTCP210x_VERSION_DEP Deprecated Confused Version Part Numbers
/// @{
#define		CP210x_CP2101_VERSION				CP210x_PARTNUM_CP2101	///< @deprecated use @ref CP210x_PARTNUM_CP2101
#define		CP210x_CP2102_VERSION				CP210x_PARTNUM_CP2102	///< @deprecated use @ref CP210x_PARTNUM_CP2102
#define		CP210x_CP2103_VERSION				CP210x_PARTNUM_CP2103	///< @deprecated use @ref CP210x_PARTNUM_CP2103
#define		CP210x_CP2104_VERSION				CP210x_PARTNUM_CP2104	///< @deprecated use @ref CP210x_PARTNUM_CP2104
#define		CP210x_CP2105_VERSION				CP210x_PARTNUM_CP2105	///< @deprecated use @ref CP210x_PARTNUM_CP2105
#define		CP210x_CP2108_VERSION				CP210x_PARTNUM_CP2108	///< @deprecated use @ref CP210x_PARTNUM_CP2108
#define     CP210x_CP2109_VERSION               CP210x_PARTNUM_CP2109	///< @deprecated use @ref CP210x_PARTNUM_CP2109
#define     CP210x_CP2102N_QFN28_VERSION		CP210x_PARTNUM_CP2102N_QFN28	///< @deprecated use @ref CP210x_PARTNUM_CP2102N_QFN28
#define     CP210x_CP2102N_QFN24_VERSION		CP210x_PARTNUM_CP2102N_QFN24	///< @deprecated use @ref CP210x_PARTNUM_CP2102N_QFN24
#define     CP210x_CP2102N_QFN20_VERSION		CP210x_PARTNUM_CP2102N_QFN20	///< @deprecated use @ref CP210x_PARTNUM_CP2102N_QFN20
/// @}


/// API Status return codes
#if defined(NOT_YET)
typedef enum _CP210x_STATUS {
	CP210x_SUCCESS = SILABS_STATUS_SUCCESS	///< Success
	, CP210x_INVALID_HANDLE = 0x01			///< an handle parameter was not valid
	, CP210x_INVALID_PARAMETER = 0x02			///< a parameter was not valid
	, CP210x_DEVICE_IO_FAILED = 0x03	///< device I/O failed
	, CP210x_FUNCTION_NOT_SUPPORTED = 0x04			///< the specified function is not supported
	, CP210x_GLOBAL_DATA_ERROR = 0x05	///< global data error
	, CP210x_FILE_ERROR = 0x06	///< file error
	, CP210x_COMMAND_FAILED = 0x08	///< command failed
	, CP210x_INVALID_ACCESS_TYPE = 0x09	///< invalid access type

	, CP210x_DEVICE_NOT_FOUND = 0xFF			///< the specified device was not found
} CP210x_STATUS, *PCP210x_STATUS;
#else // defined(NOT_YET)
/// @defgroup RTCP210x_STATUS CP210x Status Codes - runtime
/// @{
#define	CP210x_SUCCESS SILABS_STATUS_SUCCESS	///< Success
#define	CP210x_INVALID_HANDLE 0x01				///< an handle parameter was not valid
#define	CP210x_INVALID_PARAMETER 0x02			///< a parameter was not valid
#define	CP210x_DEVICE_IO_FAILED 0x03			///< device I/O failed
#define	CP210x_FUNCTION_NOT_SUPPORTED 0x04		///< the specified function is not supported
#define	CP210x_GLOBAL_DATA_ERROR 0x05			///< global data error
#define	CP210x_FILE_ERROR 0x06					///< file error
#define	CP210x_COMMAND_FAILED 0x08				///< command failed
#define	CP210x_INVALID_ACCESS_TYPE 0x09			///< invalid access type
#define	CP210x_DEVICE_NOT_FOUND 0xFF			///< the specified device was not found
typedef     int     CP210x_STATUS;	///< CP210x_STATUS
/// @}
#endif // defined(NOT_YET)

// Buffer size limits TODO: Enstone: cut'n'paste dups of CP210xManufacturingDLL.h's defines of same name
#define     CP210x_MAX_PRODUCT_STRLEN           126	///< Max Product string length
#define     CP210x_MAX_SERIAL_STRLEN_S          126	///< Max Serial Number string length

// Type definitions
typedef     char    CP210x_PRODUCT_STRING[CP210x_MAX_PRODUCT_STRLEN];
typedef     char    CP210x_SERIAL_STRING_S[CP210x_MAX_SERIAL_STRLEN_S];

/// Mask and Latch value bit definitions
/// @defgroup RTCP210x_GPIO GPIO Mask and Latch value bit definitions
/// @{
#define     CP210x_GPIO_0                       0x0001	///< GPIO pin 0
#define     CP210x_GPIO_1                       0x0002	///< GPIO pin 1
#define     CP210x_GPIO_2                       0x0004	///< GPIO pin 2
#define     CP210x_GPIO_3                       0x0008	///< GPIO pin 3
#define     CP210x_GPIO_4                       0x0010	///< GPIO pin 4
#define     CP210x_GPIO_5                       0x0020	///< GPIO pin 5
#define     CP210x_GPIO_6                       0x0040	///< GPIO pin 6
#define     CP210x_GPIO_7                       0x0080	///< GPIO pin 7
#define     CP210x_GPIO_8                       0x0100	///< GPIO pin 8
#define     CP210x_GPIO_9                       0x0200	///< GPIO pin 9
#define     CP210x_GPIO_10                      0x0400	///< GPIO pin 10
#define     CP210x_GPIO_11                      0x0800	///< GPIO pin 11
#define     CP210x_GPIO_12                      0x1000	///< GPIO pin 12
#define     CP210x_GPIO_13                      0x2000	///< GPIO pin 13
#define     CP210x_GPIO_14                      0x4000	///< GPIO pin 14
#define     CP210x_GPIO_15                      0x8000	///< GPIO pin 15
/// @}

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus
/// @brief Determines the number of CP210x devices connected to the system
///
/// @param[out] lpdwNumDevices a pointer to a uint32_t location to hold the returned device count
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref CP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_PARAMETER -- lpdwNumDevices is an unexpected value
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_GetNumDevices(
		_Out_writes_bytes_(sizeof(uint32_t)) _Pre_defensive_ uint32_t *lpdwNumDevices
	);

/// @brief Opens a handle to the device
///
/// @detail Opens and returns a handle to a device using a device index determined by the number returned
/// from @ref CP210xRT_GetNumDevices().
///
/// @param[in] DeviceIndex is the desired device's zero-based index into the device list used to determine @ref CP210xRT_GetNumDevices(). 0 for the first device, 1 for the second, etc.
/// @param[out] pcyHandle is a pointer to a HANDLE/4-byte location to hold the returned open handle to the device
///
/// The returned handle is used for all/most subsequent accesses to the device (I.e. the cyHandle parameter to all/most other API functions)
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref CP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_PARAMETER -- pcyHandle is an unexpected value
///	@retval	#CP210x_DEVICE_NOT_FOUND -- the device specified by the dwDeviceIndex was not found
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_Open(
	_In_ _Pre_defensive_ const uint32_t DeviceIndex,
	_Out_writes_bytes_(sizeof(HANDLE)) _Pre_defensive_ PHANDLE pcyHandle
);

/// @brief Closes an open handle to the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open()
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref CP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_Close(
	_In_ _Pre_defensive_ const HANDLE cyHandle
);

/// Reads and returns the current GPIO port latch value from the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open() or CreateFile()
/// @param[out] lpwLatch points at a 2-byte buffer into which the port latch value will be written
///
/// GPIO latch value is Logic High = 1, Logic Low = 0.
///
/// @note Supported on CP210x devices @ref silabs_defs.IsWriteReadLatchPartNum()
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref RTCP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
///	@retval	#CP210x_INVALID_PARAMETER -- lpwLatch is an unexpected value
///	@retval	#CP210x_DEVICE_NOT_FOUND -- the device could not be determined to be a CP210x device
///	@retval	#CP210x_FUNCTION_NOT_SUPPORTED -- the device does not support port latch
///	@retval	#CP210x_DEVICE_IO_FAILED -- the device failed to respond to I/O in any expected manner
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_ReadLatch(_In_ _Pre_defensive_ const HANDLE cyHandle,
				   _Out_writes_bytes_(sizeof(WORD)) _Pre_defensive_ LPWORD lpwLatch);

/// @brief Sets a new current GPIO port latch value for the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open() or CreateFile()
/// @param[in] mask is a bit-mask which determines which pins of @ref latch to change [Change = 1, Do not change = 0].
/// @param[in] latch is the new 2-byte GPIO port latch value to be written to the device
///
/// @note Supported on CP210x devices @ref silabs_defs.IsWriteReadLatchPartNum()
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref RTCP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
///	@retval	#CP210x_DEVICE_NOT_FOUND -- the device could not be determined to be a CP210x device
///	@retval	#CP210x_FUNCTION_NOT_SUPPORTED -- the device does not support port latch
///	@retval	#CP210x_DEVICE_IO_FAILED -- the device failed to respond to I/O in any expected manner
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_WriteLatch(_In_ _Pre_defensive_ const HANDLE cyHandle,
					_In_ _Pre_defensive_ const WORD mask,
					_In_ _Pre_defensive_ const WORD latch);

/// @brief Reads and returns the Part Number from the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open() or CreateFile()
/// @param[out] lpbPartNum points at a 1-byte buffer into which the @ref silabs_defs.SILABS_CP210X_PARTNUM Part Number value will be written
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref RTCP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
///	@retval	#CP210x_INVALID_PARAMETER -- lpbPartNum is an unexpected value
///	@retval	#CP210x_DEVICE_IO_FAILED -- the device failed to respond to I/O in any expected manner
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_GetPartNumber(_In_ _Pre_defensive_ const HANDLE cyHandle,
					   _Out_writes_bytes_(1) _Pre_defensive_ LPBYTE lpbPartNum);

/// @brief Reads and returns the Product string directly from the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open() or CreateFile()
/// @param[out] lpProductString points at a buffer into which the Product string will be written
/// @param[out] lpbProductStringLengthInBytes points at a 1-byte buffer into which the total number of bytes in the returned Product string is returned.
/// @param[in] bConvertToASCII is a true or false value indicating whether the Product string is an 16-bit UCS-2 character string needing converting to 8-bit ascii characters (true) or not (false).
///
/// lpProductString will typically be an address of a string of type @ref CP210x_PRODUCT_STRING or a similarly sized string of at least @ref CP210x_MAX_PRODUCT_STRLEN characters.
///	The returned Product string is "as is" retrieved from the device (other than converted to ascii if @ref bConvertToASCII is true).  Specifically, it may not be NULL terminated.
///
/// @note Length of lpProductString must be <= @ref CP210x_MAX_SETUP_STRING_SIZE
///
/// @note BUG: This is an string-UNSAFE function, not defensize against a "too small" buffer provided as the @ref lpProductString parameter. 
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref RTCP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
///	@retval	#CP210x_INVALID_PARAMETER -- lpProductString or lpbProductStringLengthInBytes are unexpected values
///	@retval	#CP210x_DEVICE_IO_FAILED -- the device failed to respond to I/O in any expected manner
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_GetDeviceProductString(_In_ _Pre_defensive_ const HANDLE cyHandle,
								_Out_writes_bytes_(*lpbProductStringLengthInBytes) _Pre_defensive_ LPVOID lpProductString,
								_Out_writes_bytes_(1) _Pre_defensive_ LPBYTE lpbProductStringLengthInBytes,
								_In_ _Pre_defensive_ const BOOL bConvertToASCII = TRUE
                                );

/// @brief Reads and returns the Serial Number String directly from the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open() or CreateFile()
/// @param[out] lpSerialNumberString points at a buffer into which the Serial Number String will be written
/// @param[out] lpbSerialNumberStringLengthInBytes points at a 1-byte buffer into which the total number of bytes in the returned Serial Number String is returned.
/// @param[in] bConvertToASCII is a true or false value indicating whether the Serial Number String is an 16-bit UCS-2 character string needing converting to 8-bit ascii characters (true) to not (false).
///
/// lpSerialNumberString will typically be an address of a string of type @ref CP210x_SERIAL_STRING or a similarly sized string of at least @ref CP210x_MAX_SERIAL_STRLEN characters.
///	The returned Serial Number String is "as is" retrieved from the device (other than converted to ascii if @ref bConvertToASCII is true).  Specifically, it may not be NULL terminated.
///
/// @note Length of lpSerialNumberString must be <= @ref CP210x_MAX_SETUP_STRING_SIZE
///
/// @note BUG: This is an string-UNSAFE function, not defensize against a "too small" buffer provided as the @ref lpSerialNumberString parameter. 
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref RTCP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
///	@retval	#CP210x_INVALID_PARAMETER -- lpSerialNumberString or lpbSerialNumberStringLengthInBytes are unexpected values
///	@retval	#CP210x_DEVICE_IO_FAILED -- the device failed to respond to I/O in any expected manner
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_GetDeviceSerialNumber(_In_ _Pre_defensive_ const HANDLE cyHandle,
							   _Out_writes_bytes_(*lpbSerialNumberStringLengthInBytes) _Pre_defensive_ LPVOID lpSerialNumberString,
							   _Out_writes_bytes_(1) _Pre_defensive_ LPBYTE lpbSerialNumberStringLengthInBytes,
							   _In_ _Pre_defensive_ const BOOL bConvertToASCII = TRUE
                               );

/// @brief Reads and returns the Interface String directly from the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open() or CreateFile()
/// @param[out] lpInterfaceString points at a buffer into which the Interface String will be written
/// @param[out] lpbInterfaceStringLengthInBytes points at a 1-byte buffer into which the total number of bytes in the returned Interface String is returned.
/// @param[in] bConvertToASCII is a true or false value indicating whether the Interface String is an 16-bit UCS-2 character string needing converting to 8-bit ascii characters (true) to not (false).
///
/// @ref lpInterfaceString will typically be a buffer of at least @ref TBD characters.
///	The returned Interface String is "as is" retrieved from the device (other than converted to ascii if @ref bConvertToASCII is true).  Specifically, it may not be NULL terminated.
///
/// @note Length of lpInterfaceString must be <= @ref CP210x_MAX_SETUP_STRING_SIZE
///
/// @note BUG: This is an string-UNSAFE function, not defensize against a "too small" buffer provided as the @ref lpInterfaceString parameter. 
///
/// @note supported on the @ref silabs_defs.IsMultiInterfacePartNum() CP2105 and CP2108 multi-interface CP210x devices.
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref RTCP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
///	@retval	#CP210x_INVALID_PARAMETER -- lpInterfaceString or lpbInterfaceStringLengthInBytes are unexpected values
///	@retval	#CP210x_DEVICE_IO_FAILED -- the device failed to respond to I/O in any expected manner
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_GetDeviceInterfaceString(_In_ _Pre_defensive_ const HANDLE	cyHandle,
								  _Out_writes_bytes_(*lpbInterfaceStringLengthInBytes) _Pre_defensive_ LPVOID lpInterfaceString,
								  _Out_writes_bytes_(1) _Pre_defensive_ LPBYTE lpbInterfaceStringLengthInBytes,
								  _In_ _Pre_defensive_ const BOOL bConvertToASCII);


/// @brief Reads and returns the Receiver Max Timeout directly from the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open() or CreateFile()
/// @param[out] pReceiverMaxTimeoutInMicroseconds points at a buffer into which the current Receiver Max Timeout will be written
///
/// @note xxx
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref RTCP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
///	@retval	#CP210x_INVALID_PARAMETER -- pReceiverMaxTimeoutInMicroseconds is an unexpected value
///	@retval	#CP210x_DEVICE_IO_FAILED -- the device failed to respond to I/O in any expected manner
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_GetReceiverMaxTimeout(_In_ _Pre_defensive_ const HANDLE cyHandle,
	_Out_writes_bytes_(2) _Pre_defensive_ uint16_t *pReceiverMaxTimeoutInMicroseconds);


/// @brief Sets the Receiver Max Timeout directly to the device
///
/// @param[in] cyHandle is an open handle to the device as returned by @ref CP210xRT_Open() or CreateFile()
/// @param[in] ReceiverMaxTimeoutInMicroseconds is the new Receiver Max Timeout for the device to use
///
/// @note xxx
///
/// @returns Returns #CP210x_SUCCESS on success, or another @ref RTCP210x_STATUS Return value if there is an error.
///	@retval	#CP210x_SUCCESS -- success
///	@retval	#CP210x_INVALID_HANDLE -- cyHandle is invalid
///	@retval	#CP210x_INVALID_PARAMETER -- ReceiverMaxTimeoutInMicroseconds is an unexpected value
///	@retval	#CP210x_DEVICE_IO_FAILED -- the device failed to respond to I/O in any expected manner
_Check_return_
_Ret_range_(CP210x_SUCCESS, CP210x_DEVICE_NOT_FOUND)
_Success_(return == CP210x_SUCCESS)
CP210xRUNTIMEDLL_API CP210x_STATUS WINAPI
CP210xRT_SetReceiverMaxTimeout(_In_ _Pre_defensive_ const HANDLE cyHandle,
	_In_ _Pre_defensive_ const uint16_t ReceiverMaxTimeoutInMicroseconds);

#ifdef __cplusplus
}
#endif // __cplusplus

/// @}

#endif	// !HOST_LIB_RUNTIME_INCLUDE_CP210XRUNTIMEDLL_H_INCLUDED_QU4N7WC20W


