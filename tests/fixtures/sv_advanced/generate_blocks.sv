module gen_demo #(parameter N = 4) (
    input  logic       clk,
    input  logic       en,
    input  logic [7:0] data_in  [0:N-1],
    output logic [7:0] data_out [0:N-1]
);

    // For-generate: instantiate N pipeline stages
    genvar i;
    generate
        for (i = 0; i < N; i++) begin : gen_pipe
            always_ff @(posedge clk) begin
                if (en)
                    data_out[i] <= data_in[i];
            end
        end
    endgenerate

    // If-generate: optional parity calculator
    generate
        if (N > 2) begin : gen_parity
            logic parity;
            assign parity = ^data_in[0];
        end else begin : gen_no_parity
            logic unused;
            assign unused = 1'b0;
        end
    endgenerate

    // Case-generate
    generate
        case (N)
            1: begin : gen_single
                logic single_flag;
                assign single_flag = 1'b1;
            end
            4: begin : gen_quad
                logic quad_flag;
                assign quad_flag = 1'b1;
            end
            default: begin : gen_default
                logic default_flag;
                assign default_flag = 1'b1;
            end
        endcase
    endgenerate

endmodule
