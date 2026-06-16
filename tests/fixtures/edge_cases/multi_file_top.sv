module multi_top (
    input  logic       clk,
    input  logic       rst_n,
    input  logic [7:0] data_in,
    output logic [7:0] data_out
);

    logic [7:0] stage1_out;

    other_module u_stage1 (
        .clk     (clk),
        .rst_n   (rst_n),
        .data_in (data_in),
        .data_out(stage1_out)
    );

    other_module u_stage2 (
        .clk     (clk),
        .rst_n   (rst_n),
        .data_in (stage1_out),
        .data_out(data_out)
    );

endmodule
