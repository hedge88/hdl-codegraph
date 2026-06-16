`define DATA_WIDTH 32
`define ENABLE_CHECKSUM

module ifdef_demo (
    input  logic                  clk,
    input  logic [`DATA_WIDTH-1:0] data_in,
    output logic [`DATA_WIDTH-1:0] data_out
);

    logic [`DATA_WIDTH-1:0] pipe_reg;

    always_ff @(posedge clk) begin
        pipe_reg <= data_in;
    end

    `ifdef ENABLE_CHECKSUM
        logic [7:0] checksum;
        assign checksum = ^pipe_reg;
        assign data_out = {pipe_reg[`DATA_WIDTH-1:8], checksum};
    `else
        assign data_out = pipe_reg;
    `endif

    `ifndef DISABLE_EXTRA
        logic extra_signal;
        assign extra_signal = 1'b1;
    `endif

endmodule
