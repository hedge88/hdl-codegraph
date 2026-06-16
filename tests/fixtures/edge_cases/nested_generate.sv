module nested_gen #(parameter DEPTH = 3, parameter WIDTH = 8) (
    input  logic                  clk,
    input  logic [WIDTH-1:0]      data_in,
    output logic [WIDTH-1:0]      data_out
);

    genvar i, j;

    generate
        for (i = 0; i < DEPTH; i++) begin : outer
            if (i < 2) begin : inner_if
                for (j = 0; j < 2; j++) begin : deepest
                    logic [WIDTH-1:0] stage_reg;
                    always_ff @(posedge clk) begin
                        stage_reg <= data_in;
                    end
                    if (j == 0) begin : leaf_a
                        logic leaf_flag;
                        assign leaf_flag = 1'b1;
                    end else begin : leaf_b
                        logic leaf_flag;
                        assign leaf_flag = 1'b0;
                    end
                end
            end else begin : inner_else
                logic [WIDTH-1:0] bypass;
                assign bypass = data_in;
            end
        end
    endgenerate

    assign data_out = data_in;

endmodule
