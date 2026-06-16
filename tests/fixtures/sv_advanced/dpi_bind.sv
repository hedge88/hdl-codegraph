module dpi_demo (
    input  logic        clk,
    input  logic [31:0] in_data,
    output logic [31:0] out_data
);

    // DPI-C function import
    import "DPI-C" function int unsigned c_crc32(input int unsigned data);

    // DPI-C task import
    import "DPI-C" task c_wait_cycles(input int unsigned cycles);

    // DPI-C function export
    export "DPI-C" function sv_process;

    function void sv_process(input logic [31:0] din);
        out_data = c_crc32(din);
    endfunction

    always_ff @(posedge clk) begin
        out_data <= c_crc32(in_data);
    end

endmodule

// Target module for bind
module monitor_unit (
    input logic clk,
    input logic [31:0] observed_data
);
    logic [31:0] captured;
    always_ff @(posedge clk) begin
        captured <= observed_data;
    end
endmodule

// Bind directive: bind monitor_unit dpi_bind_demo u_mon (.clk(clk), .in_data(observed_data), .out_data());

// Config block
config cfg_demo;
    design work.top;
    default liblist work;
    instance top.u0 liblist rtl_lib;
endconfig
